#version 450

// Simple UI vertex shader - Step 1 proof of concept
// Vertices are in normalized device coordinates (-1 to 1)

layout(location = 0) in vec2 inPosition;  // 2D position in NDC

void main() {
    // Pass through position directly (already in NDC)
    // Z = 0 (middle of depth range), W = 1 (standard homogeneous coord)
    gl_Position = vec4(inPosition, 0.0, 1.0);
}
