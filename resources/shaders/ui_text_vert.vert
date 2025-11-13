#version 450

// Vertex input (screen-space text quads)
layout(location = 0) in vec2 inPosition;  // NDC coordinates
layout(location = 1) in vec2 inTexCoord;  // Glyph atlas UV

// Outputs to fragment shader
layout(location = 0) out vec2 fragTexCoord;

void main() {
    // Pass through NDC position directly (no transformation needed for screen space)
    gl_Position = vec4(inPosition, 0.0, 1.0);
    
    // Pass through texture coordinates for glyph atlas sampling
    fragTexCoord = inTexCoord;
}
