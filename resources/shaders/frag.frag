#version 450

layout(push_constant) uniform PushConstants {
    mat4 mvp; // Model-View-Projection matrix
    vec4 material_color; // Material base color (RGBA)
} pushConstants;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    // Simple lighting based on normal
    vec3 lightDir = normalize(vec3(1.0, 1.0, 1.0));
    float diff = max(dot(normalize(fragNormal), lightDir), 0.0);
    
    // Use material color from push constants with simple diffuse lighting
    vec3 color = pushConstants.material_color.rgb * (0.3 + 0.7 * diff);
    fragColor = vec4(color, pushConstants.material_color.a);
}