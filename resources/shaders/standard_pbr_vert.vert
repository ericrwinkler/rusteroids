#version 450

// Vertex attributes
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;

// Instance attributes (per-instance data)
layout(location = 3) in vec4 instanceModelMatrix0;
layout(location = 4) in vec4 instanceModelMatrix1;
layout(location = 5) in vec4 instanceModelMatrix2;
layout(location = 6) in vec4 instanceModelMatrix3;
layout(location = 7) in vec4 instanceNormalMatrix0;
layout(location = 8) in vec4 instanceNormalMatrix1;
layout(location = 9) in vec4 instanceNormalMatrix2;
layout(location = 10) in vec4 instanceNormalMatrix3;
layout(location = 11) in vec4 instanceMaterialColor;
layout(location = 12) in vec4 instanceEmission;
layout(location = 13) in uvec4 instanceTextureFlags;
layout(location = 14) in uint instanceMaterialIndex;

// Camera UBO - Set 0, Binding 0
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view_matrix;
    mat4 projection_matrix;
    mat4 view_projection_matrix;
    vec4 camera_position;
    vec4 camera_direction;
    vec2 viewport_size;
    vec2 near_far;
} camera;

// Multi-Light UBO - Set 0, Binding 1
layout(set = 0, binding = 1) uniform MultiLightingUBO {
    vec4 ambient_color;                    // RGBA ambient
    uint directional_light_count;          // Number of directional lights
    uint point_light_count;                // Number of point lights  
    uint spot_light_count;                 // Number of spot lights
    uint _padding;                         // Padding for alignment
    // Light arrays follow (see fragment shader for full definition)
} lighting;

// Note: Push constants removed - using instance vertex attributes instead
// This enables true Vulkan instancing with vkCmdDrawInstanced

// Output to fragment shader
layout(location = 0) out vec3 fragPosition;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragTexCoord;
layout(location = 3) out vec3 fragCameraPosition;
layout(location = 4) out vec4 fragInstanceMaterialColor;
layout(location = 5) out flat uint fragInstanceMaterialIndex;
layout(location = 6) out vec4 fragInstanceEmission;
layout(location = 7) out flat uvec4 fragTextureFlags;

void main() {
    // Reconstruct model matrix from instance attributes
    mat4 modelMatrix = mat4(
        instanceModelMatrix0,
        instanceModelMatrix1,
        instanceModelMatrix2,
        instanceModelMatrix3
    );
    
    // Reconstruct normal matrix from instance attributes
    mat3 normalMatrix = mat3(
        instanceNormalMatrix0.xyz,
        instanceNormalMatrix1.xyz,
        instanceNormalMatrix2.xyz
    );
    
    // Transform vertex position to world space using instance model matrix
    vec4 worldPosition = modelMatrix * vec4(position, 1.0);
    
    // Transform to clip space
    gl_Position = camera.view_projection_matrix * worldPosition;
    
    // Pass world position to fragment shader
    fragPosition = worldPosition.xyz;
    
    // Transform normal to world space using instance normal matrix
    fragNormal = normalMatrix * normal;
    
    // Pass texture coordinates unchanged
    fragTexCoord = texCoord;
    
    // Pass camera position for lighting calculations
    fragCameraPosition = camera.camera_position.xyz;
    
    // Pass instance material data to fragment shader
    fragInstanceMaterialColor = instanceMaterialColor;
    fragInstanceMaterialIndex = instanceMaterialIndex;
    fragInstanceEmission = instanceEmission;
    fragTextureFlags = instanceTextureFlags;
}
