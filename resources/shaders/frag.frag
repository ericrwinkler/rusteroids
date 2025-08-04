#version 450

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    // Simple shading based on normal direction
    vec3 lightDir = normalize(vec3(1.0, 1.0, 1.0));
    float ndotl = max(dot(normalize(fragNormal), lightDir), 0.0);
    vec3 color = vec3(0.7, 0.5, 0.3) * (0.3 + 0.7 * ndotl); // Bronze color with lighting
    fragColor = vec4(color, 1.0);
}