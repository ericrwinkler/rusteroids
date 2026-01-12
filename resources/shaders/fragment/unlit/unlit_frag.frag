#version 450

// ============================================================================
// Unlit Fragment Shader
// Simple color/texture rendering without lighting calculations
// Supports: base color texture, opacity texture, alpha blending
// ============================================================================

// Include shared definitions
#include "../../common/material.glsl"

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
    
    // Sample base color texture if enabled
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
