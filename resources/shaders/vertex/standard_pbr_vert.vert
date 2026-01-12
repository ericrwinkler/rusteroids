#version 450

// ============================================================================
// Standard Vertex Shader
// Transforms vertices using instanced model/normal matrices
// Outputs world-space position, normal, tangent, and material data
// ============================================================================

// Vertex attributes
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;
layout(location = 3) in vec3 tangent;

// Include shared definitions
#include "../common/camera.glsl"
#include "../common/lighting.glsl"
#include "./instancing.glsl"

// Output to fragment shader
layout(location = 0) out vec3 fragPosition;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragTexCoord;
layout(location = 3) out vec3 fragCameraPosition;
layout(location = 4) out vec4 fragInstanceMaterialColor;
layout(location = 5) out flat uint fragInstanceMaterialIndex;
layout(location = 6) out vec4 fragInstanceEmission;
layout(location = 7) out flat uvec4 fragTextureFlags;
layout(location = 8) out vec3 fragTangent;

void main() {
    // Get instanced transformation matrices
    mat4 modelMatrix = getInstanceModelMatrix();
    mat3 normalMatrix = getInstanceNormalMatrix();
    
    // Transform vertex position to world space
    vec4 worldPosition = modelMatrix * vec4(position, 1.0);
    
    // Transform to clip space
    gl_Position = camera.view_projection_matrix * worldPosition;
    
    // Output world-space data for fragment shader
    fragPosition = worldPosition.xyz;
    fragNormal = normalMatrix * normal;
    fragTangent = normalMatrix * tangent;
    fragTexCoord = texCoord;
    fragCameraPosition = camera.camera_position.xyz;
    
    // Pass instance material data
    fragInstanceMaterialColor = instanceMaterialColor;
    fragInstanceMaterialIndex = instanceMaterialIndex;
    fragInstanceEmission = instanceEmission;
    fragTextureFlags = instanceTextureFlags;
}
