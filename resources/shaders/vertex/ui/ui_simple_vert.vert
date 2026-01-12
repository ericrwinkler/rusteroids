#version 450

// ============================================================================
// Simple UI Vertex Shader
// Renders basic UI shapes in NDC space with push constant colors
// ============================================================================

#include "../../common/ui_common.glsl"

// Vertex input - position in normalized device coordinates
layout(location = 0) in vec2 inPosition;

void main() {
    // Convert NDC position to clip space (Z=0, W=1)
    gl_Position = ndcToClipSpace(inPosition);
}
