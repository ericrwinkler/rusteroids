#version 450

// Camera UBO - Set 0, Binding 0
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// Simple Lighting UBO - Set 0, Binding 1 (per-frame updated)
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Push constants - only model matrix and material data
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    vec4 material_color;
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
    // Light direction should point FROM the light TO the surface
    float diff = max(dot(normal, -lightDir), 0.0);
    
    // Use ambient from UBO
    vec3 ambient = lighting.ambient_color.rgb * lighting.ambient_color.a;
    vec3 diffuse = diff * lightIntensity * lightColor;
    vec3 lighting_result = ambient + diffuse;
    
    // Clamp lighting to reasonable range
    lighting_result = clamp(lighting_result, 0.0, 2.0);
    
    // Apply lighting to material color from push constants
    vec3 color = pushConstants.material_color.rgb * lighting_result;
    fragColor = vec4(color, pushConstants.material_color.a);
}
