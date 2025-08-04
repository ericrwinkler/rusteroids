#version 450

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    // Simple lighting based on normal
    vec3 lightDir = normalize(vec3(1.0, 1.0, 1.0));
    float diff = max(dot(normalize(fragNormal), lightDir), 0.0);
    
    // Bronze-ish color with simple diffuse lighting
    vec3 color = vec3(0.8, 0.6, 0.3) * (0.3 + 0.7 * diff);
    fragColor = vec4(color, 1.0);
}