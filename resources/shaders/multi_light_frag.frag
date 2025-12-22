#version 450

// Maximum light limits (must match Rust constants)
#define MAX_DIRECTIONAL_LIGHTS 4
#define MAX_POINT_LIGHTS 64
#define MAX_SPOT_LIGHTS 4

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

// Multi-Light UBO - Set 0, Binding 1 (Phase 3 Step 3.1: Enable Multiple Lights)
// This structure must exactly match MultiLightingUBO in ubo_manager.rs
layout(set = 0, binding = 1) uniform MultiLightUBO {
    vec4 ambient_color;                                        // 16 bytes - RGBA ambient
    uint directional_light_count;                              // 4 bytes
    uint point_light_count;                                    // 4 bytes
    uint spot_light_count;                                     // 4 bytes
    uint _padding;                                             // 4 bytes
    DirectionalLightData directional_lights[MAX_DIRECTIONAL_LIGHTS];  // 4 * 32 = 128 bytes
    PointLightData point_lights[MAX_POINT_LIGHTS];             // 64 * 48 = 3072 bytes
    SpotLightData spot_lights[MAX_SPOT_LIGHTS];               // 4 * 64 = 256 bytes
} lighting;

// Push constants - unchanged from current shader
layout(push_constant) uniform PushConstants {
    mat4 model_matrix;
    mat3 normal_matrix;
    vec4 material_color;
} pushConstants;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;
layout(location = 2) in vec3 fragWorldPos;

layout(location = 0) out vec4 fragColor;

void main() {
    vec3 normal = normalize(fragNormal);
    
    // Phase 3 Step 3.1: Process multiple lights following Vulkano tutorial pattern
    
    // Start with ambient lighting
    vec3 color = lighting.ambient_color.rgb * lighting.ambient_color.a;
    
    // Process all directional lights
    for (uint i = 0u; i < lighting.directional_light_count && i < MAX_DIRECTIONAL_LIGHTS; ++i) {
        DirectionalLightData light = lighting.directional_lights[i];
        vec3 lightDir = normalize(light.direction.xyz);
        
        // Try both directions to see which works
        float diff1 = max(dot(normal, lightDir), 0.0);
        float diff2 = max(dot(normal, -lightDir), 0.0);
        float diff = max(diff1, diff2); // Use whichever gives more light
        
        vec3 diffuse = diff * light.direction.w * light.color.rgb; // direction.w = intensity
        color += diffuse;
    }
    
    // Process all point lights with proper distance attenuation
    for (uint i = 0u; i < lighting.point_light_count && i < MAX_POINT_LIGHTS; ++i) {
        PointLightData light = lighting.point_lights[i];
        
        // Calculate distance from fragment to light
        vec3 lightPos = light.position.xyz;
        vec3 lightDir = lightPos - fragWorldPos;
        float distance = length(lightDir);
        float range = light.position.w; // position.w contains range
        
        // Apply range cutoff - no contribution beyond range
        if (distance > range) {
            continue;
        }
        
        // Normalize light direction
        lightDir = normalize(lightDir);
        
        // Calculate attenuation based on distance and range
        // Using smooth falloff: intensity * (1 - (distance/range)^2)^2
        float attenuation = 1.0 - (distance / range);
        attenuation = attenuation * attenuation; // Smooth quadratic falloff
        
        // Calculate basic lambertian lighting
        float ndotl = max(dot(normal, lightDir), 0.0);
        
        vec3 lightContrib = light.color.rgb * light.color.w * attenuation * ndotl;
        color += lightContrib;
    }
    
    // Process all spot lights
    for (uint i = 0u; i < lighting.spot_light_count && i < MAX_SPOT_LIGHTS; ++i) {
        SpotLightData light = lighting.spot_lights[i];
        // Similar to point lights - simplified until fragment position available
        vec3 lightContrib = light.color.rgb * light.direction.w; // direction.w = intensity
        color += lightContrib * 0.3; // Reduced contribution
    }
    
    // Clamp lighting to reasonable range
    color = clamp(color, 0.0, 2.0);
    
    // Apply lighting to material color
    vec3 final_color = pushConstants.material_color.rgb * color;
    fragColor = vec4(final_color, pushConstants.material_color.a);
}
