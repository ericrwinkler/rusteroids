#version 450

// Material UBO - Set 1, Binding 0
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;                        // RGB + alpha
    vec4 metallic_roughness_ao_normal;      // metallic, roughness, ao, normal_scale
    vec4 emission;                          // RGB emission + strength
    uvec4 texture_flags;                    // base_color, normal, metallic_roughness, ao
    vec4 additional_params;                 // reserved for future use
    vec4 _padding;
} material;

// Textures - Set 1, Bindings 1-4
layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;
layout(set = 1, binding = 2) uniform sampler2D normalTexture;
layout(set = 1, binding = 3) uniform sampler2D metallicRoughnessTexture;
layout(set = 1, binding = 4) uniform sampler2D aoTexture;

// Lighting UBO - Set 0, Binding 1
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Input from vertex shader
layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragCameraPosition;

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
    
    // Geometry term (Smith model)
    float k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    float G1L = NdotL / (NdotL * (1.0 - k) + k);
    float G1V = NdotV / (NdotV * (1.0 - k) + k);
    float G = G1L * G1V;
    
    // Cook-Torrance BRDF
    vec3 numerator = D * G * F;
    float denominator = 4.0 * NdotV * NdotL + 0.0001; // Add small value to prevent divide by zero
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
    // Sample base color
    vec3 baseColor = material.base_color.rgb;
    float alpha = material.base_color.a;
    
    if (material.texture_flags.x != 0u) {
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
    
    // Sample normal map (simplified)
    vec3 normal = normalize(fragNormal);
    if (material.texture_flags.y != 0u) {
        // TODO: Implement proper tangent space normal mapping
        // For now, just use the vertex normal
    }
    
    // Lighting calculations
    vec3 lightDir = normalize(-lighting.directional_light_direction.xyz);
    vec3 viewDir = normalize(fragCameraPosition - fragPosition);
    vec3 lightColor = lighting.directional_light_color.rgb * lighting.directional_light_direction.w;
    
    // Calculate lighting
    vec3 ambient = lighting.ambient_color.rgb * lighting.ambient_color.a * baseColor * ao;
    vec3 lighting_result = ambient + calculatePBR(baseColor, metallic, roughness, normal, lightDir, viewDir, lightColor);
    
    // Add emission
    vec3 emission = material.emission.rgb * material.emission.a;
    lighting_result += emission;
    
    // Output final color
    fragColor = vec4(lighting_result, alpha);
}
