#version 450

// Set 0: Per-frame data
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Set 1: Per-material data
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;                    // Base color (albedo) RGBA
    vec4 metallic_roughness_ao_normal;  // metallic, roughness, AO, normal_scale
    vec4 emission;                      // RGB emission + strength
    uvec4 texture_flags;                // Which textures are bound
    vec4 additional_params;             // Reserved for future use
    vec4 _padding;                      // Ensure alignment
} material;

// Set 1: Texture samplers (for future use)
layout(set = 1, binding = 1) uniform sampler2D base_color_texture;
layout(set = 1, binding = 2) uniform sampler2D normal_texture;
layout(set = 1, binding = 3) uniform sampler2D metallic_roughness_texture;

// Push constants - minimal data
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
} pushConstants;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    // Extract lighting parameters from per-frame UBO
    vec3 lightDir = normalize(lighting.directional_light_direction.xyz);
    float lightIntensity = lighting.directional_light_direction.w;
    vec3 lightColor = lighting.directional_light_color.rgb;
    
    // Calculate diffuse lighting
    vec3 normal = normalize(fragNormal);
    float diff = max(dot(normal, -lightDir), 0.0);
    
    // Use ambient from UBO
    vec3 ambient = lighting.ambient_color.rgb * lighting.ambient_color.a;
    vec3 diffuse = diff * lightIntensity * lightColor;
    vec3 lighting_result = ambient + diffuse;
    
    // Use material data from Material UBO
    vec3 base_color = material.base_color.rgb;
    float alpha = material.base_color.a;
    
    // Basic material application (will be expanded for PBR)
    vec3 final_color = base_color * lighting_result;
    
    // Add emission
    final_color += material.emission.rgb * material.emission.a;
    
    // Clamp to reasonable range
    final_color = clamp(final_color, 0.0, 10.0);
    
    fragColor = vec4(final_color, alpha);
}
