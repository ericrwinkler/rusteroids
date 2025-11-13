#version 450

// Fragment input from vertex shader
layout(location = 0) in vec2 fragTexCoord;

// Output color
layout(location = 0) out vec4 outColor;

// Glyph atlas texture (single-channel alpha)
layout(set = 0, binding = 0) uniform sampler2D glyphAtlas;

// Push constants for text color
layout(push_constant) uniform PushConstants {
    vec4 textColor;  // RGBA color for text
} pushConstants;

void main() {
    // Sample glyph atlas (alpha channel contains glyph coverage)
    float alpha = texture(glyphAtlas, fragTexCoord).a;
    
    // Output text color with atlas alpha
    outColor = vec4(pushConstants.textColor.rgb, pushConstants.textColor.a * alpha);
}
