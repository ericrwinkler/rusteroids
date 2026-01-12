// Instance attribute definitions
// These are per-instance data provided by the Vulkan instancing system

// Instance model matrix (4x vec4 = mat4)
layout(location = 4) in vec4 instanceModelMatrix0;
layout(location = 5) in vec4 instanceModelMatrix1;
layout(location = 6) in vec4 instanceModelMatrix2;
layout(location = 7) in vec4 instanceModelMatrix3;

// Instance normal matrix (3x vec4, using xyz only = mat3)
layout(location = 8) in vec4 instanceNormalMatrix0;
layout(location = 9) in vec4 instanceNormalMatrix1;
layout(location = 10) in vec4 instanceNormalMatrix2;
layout(location = 11) in vec4 instanceNormalMatrix3;

// Instance material properties
layout(location = 12) in vec4 instanceMaterialColor;
layout(location = 13) in vec4 instanceEmission;
layout(location = 14) in uvec4 instanceTextureFlags;
layout(location = 15) in uint instanceMaterialIndex;

// Helper functions to reconstruct matrices
mat4 getInstanceModelMatrix() {
    return mat4(
        instanceModelMatrix0,
        instanceModelMatrix1,
        instanceModelMatrix2,
        instanceModelMatrix3
    );
}

mat3 getInstanceNormalMatrix() {
    return mat3(
        instanceNormalMatrix0.xyz,
        instanceNormalMatrix1.xyz,
        instanceNormalMatrix2.xyz
    );
}
