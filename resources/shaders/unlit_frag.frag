#version 450

// Material UBO - Set 1, Binding 0
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 color;                    // RGB + alpha
    uvec4 texture_flags;           // base_color, unused, unused, unused
    vec4 additional_params;        // reserved for future use
    vec4 _padding;
} material;

// Base color texture - Set 1, Binding 1
layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;

// Input from vertex shader
layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragCameraPosition;

// Output color
layout(location = 0) out vec4 fragColor;

void main() {
    // Sample base color
    vec3 color = material.color.rgb;
    float alpha = material.color.a;
    
    if (material.texture_flags.x != 0u) {
        vec4 textureColor = texture(baseColorTexture, fragTexCoord);
        color *= textureColor.rgb;
        alpha *= textureColor.a;
    }
    
    // Output unlit color
    fragColor = vec4(color, alpha);
}
