#version 450

// Material UBO - Set 1, Binding 0
// Use same structure as standard_pbr_frag for compatibility
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;                        // RGB + alpha
    vec4 metallic_roughness_ao_normal;      // metallic, roughness, ao, normal_scale (unused)
    vec4 emission;                          // RGB emission + strength (unused)
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

// Input from vertex shader
layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragCameraPosition;
layout(location = 4) in vec4 fragInstanceMaterialColor;
layout(location = 5) in flat uint fragInstanceMaterialIndex;

// Output color
layout(location = 0) out vec4 fragColor;

void main() {
    // Use instance material color (per-object color from instance buffer)
    vec3 color = fragInstanceMaterialColor.rgb;
    float alpha = fragInstanceMaterialColor.a;
    
    // Sample texture if enabled and multiply with instance color
    if (material.texture_flags.x != 0u) {
        vec4 textureColor = texture(baseColorTexture, fragTexCoord);
        color *= textureColor.rgb;
        alpha *= textureColor.a;
    }
    
    // Apply opacity texture if enabled
    if (material.additional_params.y != 0.0) {
        alpha *= texture(opacityTexture, fragTexCoord).r;
    }
    
    // Output unlit color (no lighting calculations)
    fragColor = vec4(color, alpha);
}
