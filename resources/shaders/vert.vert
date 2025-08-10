#version 450

layout(push_constant) uniform PushConstants {
    mat4 mvp; // Model-View-Projection matrix
    vec4 material_color; // Material base color (RGBA)
} pushConstants;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragTexCoord;

void main() {
    gl_Position = pushConstants.mvp * vec4(inPosition, 1.0);
    fragNormal = inNormal; // For now, don't transform normals
    fragTexCoord = inTexCoord;
}