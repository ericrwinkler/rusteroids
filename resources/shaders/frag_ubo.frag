#version 450

// Per-frame uniform buffer (set 0, binding 0)
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// Per-frame lighting uniform buffer (set 0, binding 1)
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    // Directional lights (up to 4)
    vec4 directional_lights_direction[4];
    vec4 directional_lights_color[4];
    // Point lights (up to 8)
    vec4 point_lights_position[8];
    vec4 point_lights_color[8];
    vec4 point_lights_attenuation[8];
    // Spot lights (up to 4)
    vec4 spot_lights_position[4];
    vec4 spot_lights_direction[4];
    vec4 spot_lights_color[4];
    vec4 spot_lights_params[4];
    // Light counts
    uint num_dir_lights;
    uint num_point_lights;
    uint num_spot_lights;
    uint _padding;
} lighting;

// Per-material uniform buffer (set 1, binding 0)
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    vec4 metallic_roughness; // metallic, roughness, ao, _padding
    vec4 emission;
    float normal_scale;
    uint texture_flags;
    uint _padding1;
    uint _padding2;
} material;

// Material textures
layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;
layout(set = 1, binding = 2) uniform sampler2D normalTexture;
layout(set = 1, binding = 3) uniform sampler2D metallicRoughnessTexture;

// Fragment inputs from vertex shader
layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragViewPos;

// Output
layout(location = 0) out vec4 fragColor;

// Texture usage flags
const uint USE_BASE_COLOR_TEXTURE = 1u;
const uint USE_NORMAL_TEXTURE = 2u;
const uint USE_METALLIC_ROUGHNESS_TEXTURE = 4u;

// PBR lighting calculation
vec3 calculateDirectionalLight(uint lightIndex, vec3 normal, vec3 viewDir, vec3 albedo, float metallic, float roughness) {
    vec3 lightDir = normalize(-lighting.directional_lights_direction[lightIndex].xyz);
    vec3 lightColor = lighting.directional_lights_color[lightIndex].rgb;
    float lightIntensity = lighting.directional_lights_color[lightIndex].a;
    
    // Simple Lambertian diffuse
    float NdotL = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = NdotL * lightColor * lightIntensity;
    
    // Simple Blinn-Phong specular (approximation for now)
    vec3 halfwayDir = normalize(lightDir + viewDir);
    float NdotH = max(dot(normal, halfwayDir), 0.0);
    float specularStrength = pow(NdotH, mix(32.0, 2.0, roughness));
    vec3 specular = specularStrength * lightColor * lightIntensity * metallic;
    
    return mix(diffuse, specular, metallic) * albedo;
}

vec3 calculatePointLight(uint lightIndex, vec3 normal, vec3 viewDir, vec3 albedo, float metallic, float roughness) {
    vec3 lightPos = lighting.point_lights_position[lightIndex].xyz;
    vec3 lightColor = lighting.point_lights_color[lightIndex].rgb;
    float lightIntensity = lighting.point_lights_color[lightIndex].a;
    vec3 attenuation = lighting.point_lights_attenuation[lightIndex].xyz;
    
    vec3 lightDir = normalize(lightPos - fragWorldPos);
    float distance = length(lightPos - fragWorldPos);
    
    // Attenuation calculation
    float attenuationFactor = 1.0 / (attenuation.x + attenuation.y * distance + attenuation.z * distance * distance);
    
    // Simple Lambertian diffuse
    float NdotL = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = NdotL * lightColor * lightIntensity * attenuationFactor;
    
    // Simple Blinn-Phong specular
    vec3 halfwayDir = normalize(lightDir + viewDir);
    float NdotH = max(dot(normal, halfwayDir), 0.0);
    float specularStrength = pow(NdotH, mix(32.0, 2.0, roughness));
    vec3 specular = specularStrength * lightColor * lightIntensity * attenuationFactor * metallic;
    
    return mix(diffuse, specular, metallic) * albedo;
}

void main() {
    // Sample material properties
    vec3 albedo = material.base_color.rgb;
    if ((material.texture_flags & USE_BASE_COLOR_TEXTURE) != 0u) {
        albedo *= texture(baseColorTexture, fragTexCoord).rgb;
    }
    
    float metallic = material.metallic_roughness.x;
    float roughness = material.metallic_roughness.y;
    if ((material.texture_flags & USE_METALLIC_ROUGHNESS_TEXTURE) != 0u) {
        vec3 metallicRoughnessSample = texture(metallicRoughnessTexture, fragTexCoord).rgb;
        metallic *= metallicRoughnessSample.b; // Blue channel = metallic
        roughness *= metallicRoughnessSample.g; // Green channel = roughness
    }
    
    // Get normal
    vec3 normal = normalize(fragNormal);
    if ((material.texture_flags & USE_NORMAL_TEXTURE) != 0u) {
        // Simple normal mapping (could be improved with proper tangent space)
        vec3 normalSample = texture(normalTexture, fragTexCoord).rgb * 2.0 - 1.0;
        normal = normalize(normal + normalSample * material.normal_scale);
    }
    
    // Calculate view direction
    vec3 viewDir = normalize(camera.camera_position.xyz - fragWorldPos);
    
    // Start with ambient lighting
    vec3 color = lighting.ambient_color.rgb * lighting.ambient_color.a * albedo;
    
    // Add directional lights
    for (uint i = 0u; i < lighting.num_dir_lights && i < 4u; ++i) {
        color += calculateDirectionalLight(i, normal, viewDir, albedo, metallic, roughness);
    }
    
    // Add point lights
    for (uint i = 0u; i < lighting.num_point_lights && i < 8u; ++i) {
        color += calculatePointLight(i, normal, viewDir, albedo, metallic, roughness);
    }
    
    // Add emission
    color += material.emission.rgb * material.emission.a;
    
    // Simple tone mapping and gamma correction
    color = color / (color + vec3(1.0)); // Reinhard tone mapping
    color = pow(color, vec3(1.0/2.2));   // Gamma correction
    
    fragColor = vec4(color, material.base_color.a);
}
