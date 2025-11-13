#version 450

// Simple UI fragment shader - Step 1 proof of concept
// Outputs solid color via push constants

layout(push_constant) uniform PushConstants {
    vec4 color;  // RGBA color
} pushConstants;

layout(location = 0) out vec4 fragColor;

void main() {
    fragColor = pushConstants.color;
}
