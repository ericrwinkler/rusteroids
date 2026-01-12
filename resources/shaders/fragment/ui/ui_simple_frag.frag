#version 450

// ============================================================================
// Simple UI Fragment Shader
// Outputs solid color from push constants
// ============================================================================

// Push constants for color
layout(push_constant) uniform PushConstants {
    vec4 color;  // RGBA color
} pushConstants;

// Output color
layout(location = 0) out vec4 fragColor;

void main() {
    fragColor = pushConstants.color;
}
