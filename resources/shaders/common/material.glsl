// Material UBO - Set 1, Binding 0
// Shared material data structure for all material types
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;                        // RGB + alpha
    vec4 metallic_roughness_ao_normal;      // metallic, roughness, ao, normal_scale
    vec4 emission;                          // RGB emission + strength
    uvec4 texture_flags;                    // base_color, normal, metallic_roughness, ao
    vec4 additional_params;                 // x: emission_texture_flag, y: opacity_texture_flag, z: unused, w: unused
    vec4 _padding;
} material;

// Texture bindings - Set 1, Bindings 1-6
layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;
layout(set = 1, binding = 2) uniform sampler2D normalTexture;
layout(set = 1, binding = 3) uniform sampler2D metallicRoughnessTexture;
layout(set = 1, binding = 4) uniform sampler2D aoTexture;
layout(set = 1, binding = 5) uniform sampler2D emissionTexture;
layout(set = 1, binding = 6) uniform sampler2D opacityTexture;
