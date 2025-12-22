#version 450

// Material UBO - Set 1, Binding 0
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;                        // RGB + alpha
    vec4 metallic_roughness_ao_normal;      // metallic, roughness, ao, normal_scale
    vec4 emission;                          // RGB emission + strength
    uvec4 texture_flags;                    // base_color, normal, metallic_roughness, ao
    vec4 additional_params;                 // x: emission_texture_flag, y: opacity_texture_flag, z: unused, w: unused
    vec4 _padding;
} material;

// Textures - Set 1, Bindings 1-6
layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;
layout(set = 1, binding = 2) uniform sampler2D normalTexture;
layout(set = 1, binding = 3) uniform sampler2D metallicRoughnessTexture;
layout(set = 1, binding = 4) uniform sampler2D aoTexture;
layout(set = 1, binding = 5) uniform sampler2D emissionTexture;
layout(set = 1, binding = 6) uniform sampler2D opacityTexture;

// Light structure definitions
struct DirectionalLight {
    vec4 direction;    // xyz + intensity
    vec4 color;        // rgb + padding
};

struct PointLight {
    vec4 position;     // xyz + range
    vec4 color;        // rgb + intensity
    vec4 attenuation;  // constant, linear, quadratic, padding
};

struct SpotLight {
    vec4 position;     // xyz + range
    vec4 direction;    // xyz + intensity
    vec4 color;        // rgb + padding
    vec4 cone_angles;  // inner, outer, unused, unused
};

// Multi-Light UBO - Set 0, Binding 1 (matches MultiLightingUBO in Rust)
layout(set = 0, binding = 1) uniform MultiLightingUBO {
    vec4 ambient_color;                    // RGBA ambient
    uint directional_light_count;          // Number of directional lights
    uint point_light_count;                // Number of point lights  
    uint spot_light_count;                 // Number of spot lights
    uint _padding;                         // Padding for alignment
    
    DirectionalLight directional_lights[4]; // Directional lights (up to 4)
    PointLight point_lights[64];              // Point lights (up to 64)
    SpotLight spot_lights[4];                // Spot lights (up to 4)
} lighting;

// Input from vertex shader
layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragCameraPosition;
layout(location = 4) in vec4 fragInstanceMaterialColor;
layout(location = 5) in flat uint fragInstanceMaterialIndex;
layout(location = 6) in vec4 fragInstanceEmission;
layout(location = 7) in flat uvec4 fragTextureFlags;
layout(location = 8) in vec3 fragTangent;

// Output color
layout(location = 0) out vec4 fragColor;

// Simple PBR lighting calculation
vec3 calculatePBR(vec3 albedo, float metallic, float roughness, vec3 normal, vec3 lightDir, vec3 viewDir, vec3 lightColor) {
    // Improved PBR with more pronounced material differences
    float NdotL = max(dot(normal, lightDir), 0.0);
    float NdotV = max(dot(normal, viewDir), 0.0);
    vec3 halfDir = normalize(lightDir + viewDir);
    float NdotH = max(dot(normal, halfDir), 0.0);
    float VdotH = max(dot(viewDir, halfDir), 0.0);
    
    // Fresnel reflectance at normal incidence (for dielectrics use 0.04, for metals use albedo)
    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    
    // Fresnel term (Schlick approximation)
    vec3 F = F0 + (1.0 - F0) * pow(clamp(1.0 - VdotH, 0.0, 1.0), 5.0);
    
    // Distribution term (GGX/Trowbridge-Reitz)
    float alpha = roughness * roughness;
    float alpha2 = alpha * alpha;
    float denom = NdotH * NdotH * (alpha2 - 1.0) + 1.0;
    float D = alpha2 / (3.14159265 * denom * denom);
    
    // Geometry term (Smith model) with better numerical stability
    float k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    float G1L = NdotL / max(NdotL * (1.0 - k) + k, 0.001); // Prevent division by zero
    float G1V = NdotV / max(NdotV * (1.0 - k) + k, 0.001); // Prevent division by zero
    float G = G1L * G1V;
    
    // Cook-Torrance BRDF with better numerical stability
    vec3 numerator = D * G * F;
    float denominator = max(4.0 * NdotV * NdotL, 0.001); // Better minimum threshold
    vec3 specular = numerator / denominator;
    
    // Diffuse term (Lambertian)
    // For energy conservation, diffuse should be reduced by the specular contribution
    vec3 kS = F; // Specular reflection coefficient
    vec3 kD = vec3(1.0) - kS; // Diffuse reflection coefficient
    kD *= 1.0 - metallic; // Metallic surfaces have no diffuse lighting
    
    vec3 diffuse = kD * albedo / 3.14159265;
    
    return (diffuse + specular) * lightColor * NdotL;
}

void main() {
    // Sample base color and blend with instance material color
    vec3 baseColor = material.base_color.rgb * fragInstanceMaterialColor.rgb;
    float alpha = material.base_color.a * fragInstanceMaterialColor.a;
    
    // Apply base color texture if enabled per-instance
    if (fragTextureFlags.x != 0u) {
        vec4 textureColor = texture(baseColorTexture, fragTexCoord);
        baseColor *= textureColor.rgb;
        alpha *= textureColor.a;
    }
    
    // Sample material properties
    float metallic = material.metallic_roughness_ao_normal.x;
    float roughness = material.metallic_roughness_ao_normal.y;
    float ao = material.metallic_roughness_ao_normal.z;
    
    if (material.texture_flags.z != 0u) {
        vec3 metallicRoughness = texture(metallicRoughnessTexture, fragTexCoord).rgb;
        metallic *= metallicRoughness.b;  // Blue channel
        roughness *= metallicRoughness.g; // Green channel
    }
    
    if (material.texture_flags.w != 0u) {
        ao *= texture(aoTexture, fragTexCoord).r;
    }
    
    // Sample normal map using per-instance flag with proper tangent-space transformation
    vec3 normal = normalize(fragNormal);
    if (fragTextureFlags.y != 0u) {
        // Build TBN (Tangent-Bitangent-Normal) matrix for tangent space to world space transformation
        vec3 T = normalize(fragTangent);
        vec3 N = normalize(fragNormal);
        
        // Gram-Schmidt orthogonalization to ensure T is perpendicular to N
        T = normalize(T - dot(T, N) * N);
        
        // Calculate bitangent (cross product of normal and tangent)
        vec3 B = cross(N, T);
        
        // Construct TBN matrix (transforms from tangent space to world space)
        mat3 TBN = mat3(T, B, N);
        
        // Sample normal map and convert from [0,1] to [-1,1] range
        vec3 normalMap = texture(normalTexture, fragTexCoord).rgb;
        normalMap = normalMap * 2.0 - 1.0;
        
        // Transform normal from tangent space to world space
        normal = normalize(TBN * normalMap);
    }
    
    // Multi-Light calculations
    vec3 viewDir = normalize(fragCameraPosition - fragPosition);
    
    // Start with ambient lighting
    vec3 ambient = lighting.ambient_color.rgb * lighting.ambient_color.a * baseColor * ao;
    vec3 lighting_result = ambient;
    
    // Process directional lights
    for (uint i = 0u; i < lighting.directional_light_count && i < 4u; i++) {
        vec3 lightDir = normalize(-lighting.directional_lights[i].direction.xyz); // Light direction should point FROM light TO surface
        vec3 lightColor = lighting.directional_lights[i].color.rgb;
        float lightIntensity = lighting.directional_lights[i].direction.w; // Intensity stored in direction.w
        lighting_result += calculatePBR(baseColor, metallic, roughness, normal, lightDir, viewDir, lightColor * lightIntensity);
    }
    
    // Process point lights
    for (uint i = 0u; i < lighting.point_light_count && i < 64u; i++) {
        vec3 lightPos = lighting.point_lights[i].position.xyz;
        float lightRange = lighting.point_lights[i].position.w;
        vec3 lightColor = lighting.point_lights[i].color.rgb; // Don't double-multiply intensity - it's already in color.w for attenuation
        float lightIntensity = lighting.point_lights[i].color.w;
        
        vec3 lightDir = lightPos - fragPosition;
        float distance = length(lightDir);
        
        // Skip if beyond range
        if (distance > lightRange) continue;
        
        lightDir = normalize(lightDir);
        
        // Calculate attenuation with improved falloff
        float constant = lighting.point_lights[i].attenuation.x;
        float linear = lighting.point_lights[i].attenuation.y;
        float quadratic = lighting.point_lights[i].attenuation.z;
        
        // Standard inverse square attenuation
        float attenuation = 1.0 / (constant + linear * distance + quadratic * distance * distance);
        
        // Add smooth falloff near range limit to prevent harsh cutoff
        float rangeFactor = 1.0 - smoothstep(lightRange * 0.7, lightRange, distance);
        attenuation *= rangeFactor;
        
        vec3 attenuatedColor = lightColor * lightIntensity * attenuation;
        lighting_result += calculatePBR(baseColor, metallic, roughness, normal, lightDir, viewDir, attenuatedColor);
    }
    
    // Process spot lights
    for (uint i = 0u; i < lighting.spot_light_count && i < 4u; i++) {
        vec3 lightPos = lighting.spot_lights[i].position.xyz;
        float lightRange = lighting.spot_lights[i].position.w;
        vec3 lightDirection = normalize(lighting.spot_lights[i].direction.xyz);
        float lightIntensity = lighting.spot_lights[i].direction.w;
        vec3 lightColor = lighting.spot_lights[i].color.rgb * lightIntensity;
        
        vec3 lightDir = lightPos - fragPosition;
        float distance = length(lightDir);
        
        // Skip if beyond range
        if (distance > lightRange) continue;
        
        lightDir = normalize(lightDir);
        
        // Calculate spot light cone
        float innerCone = lighting.spot_lights[i].cone_angles.x;
        float outerCone = lighting.spot_lights[i].cone_angles.y;
        float theta = dot(lightDir, -lightDirection);
        float epsilon = innerCone - outerCone;
        float intensity = clamp((theta - outerCone) / epsilon, 0.0, 1.0);
        
        // Skip if outside cone
        if (intensity <= 0.0) continue;
        
        vec3 attenuatedColor = lightColor * intensity;
        lighting_result += calculatePBR(baseColor, metallic, roughness, normal, lightDir, viewDir, attenuatedColor);
    }
    
    // Add emission (use per-instance emission from vertex attributes)
    vec3 emissionColor = fragInstanceEmission.rgb;
    float emissionStrength = fragInstanceEmission.a;
    
    // Sample emission texture if enabled (additional_params.x is flag)
    if (material.additional_params.x != 0.0) {
        vec4 emissionTex = texture(emissionTexture, fragTexCoord);
        emissionColor *= emissionTex.rgb * emissionTex.a; // Use alpha as emission mask
    }
    
    lighting_result += emissionColor * emissionStrength;
    
    // Apply opacity texture if enabled (additional_params.y is flag)
    float finalAlpha = alpha;
    if (material.additional_params.y != 0.0) {
        finalAlpha *= texture(opacityTexture, fragTexCoord).r;
    }
    
    // Alpha cutoff for transparent materials (like text with font atlas)
    // Discard fragments below threshold to prevent rendering background areas
    if (finalAlpha < 0.1) {
        discard;
    }
    
    // Output final color
    fragColor = vec4(lighting_result, finalAlpha);
}
