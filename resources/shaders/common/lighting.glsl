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

// Multi-Light UBO - Set 0, Binding 1
// Available in both vertex and fragment shaders
layout(set = 0, binding = 1) uniform MultiLightingUBO {
    vec4 ambient_color;                    // RGBA ambient
    uint directional_light_count;          // Number of directional lights
    uint point_light_count;                // Number of point lights  
    uint spot_light_count;                 // Number of spot lights
    uint _padding;                         // Padding for alignment
    
    DirectionalLight directional_lights[4]; // Directional lights (up to 4)
    PointLight point_lights[64];            // Point lights (up to 64)
    SpotLight spot_lights[4];               // Spot lights (up to 4)
} lighting;
