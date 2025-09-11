#version 450

// Light data structures (must be defined before UBO usage)
struct DirectionalLightData {
    vec4 direction;        // xyz + intensity
    vec4 color;           // rgb + padding
};

struct PointLightData {
    vec4 position;        // xyz + range
    vec4 color;           // rgb + intensity
    vec4 attenuation;     // constant, linear, quadratic, padding
};

struct SpotLightData {
    vec4 position;        // xyz + range
    vec4 direction;       // xyz + intensity
    vec4 color;           // rgb + padding
    vec4 cone_angles;     // inner_angle, outer_angle, unused, unused
};

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

// TEMPORARY: Use current SimpleLightingUBO for Step 1.2 validation
// This matches the existing UBO structure exactly for visual comparison
layout(set = 0, binding = 1) uniform LightingUBO {
    vec4 ambient_color;
    vec4 directional_light_direction;
    vec4 directional_light_color;
    vec4 _padding;
} lighting;

// Push constants - unchanged from current shader
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    vec4 material_color;
} pushConstants;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    vec3 normal = normalize(fragNormal);
    
    // TEMPORARY: Use current SimpleLightingUBO structure for Step 1.2 validation
    // This should produce IDENTICAL results to frag_ubo_simple.frag
    
    // Start with ambient (same as current shader)
    vec3 color = lighting.ambient_color.rgb * lighting.ambient_color.a;
    
    // Apply directional light (same calculation as current shader)
    vec3 lightDir = normalize(lighting.directional_light_direction.xyz);
    float diff = max(dot(normal, -lightDir), 0.0);
    vec3 diffuse = diff * lighting.directional_light_color.a * lighting.directional_light_color.rgb;
    
    color += diffuse;
    
    // Clamp lighting to reasonable range (same as current shader)
    color = clamp(color, 0.0, 2.0);
    
    // Apply lighting to material color (identical to current shader)
    vec3 final_color = pushConstants.material_color.rgb * color;
    fragColor = vec4(final_color, pushConstants.material_color.a);
}
