#version 450

// ============================================================================
// UI Text Vertex Shader
// Renders text glyphs from atlas with texture coordinates
// ============================================================================

#include "../../common/ui_common.glsl"

// Vertex input
layout(location = 0) in vec2 inPosition;   // NDC coordinates
layout(location = 1) in vec2 inTexCoord;   // Glyph atlas UV

// Output to fragment shader
layout(location = 0) out vec2 fragTexCoord;

void main() {
    // Convert NDC position to clip space (Z=0, W=1)
    gl_Position = ndcToClipSpace(inPosition);
    
    // Pass through texture coordinates for glyph atlas sampling
    fragTexCoord = inTexCoord;
}
