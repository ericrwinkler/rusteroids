#version 450

layout(push_constant) uniform PushConstants {
    mat4 mvp; // Model-View-Projection matrix
    mat3 normal_matrix; // Normal transformation matrix (padded to 48 bytes in memory)
    vec4 material_color; // Material base color (RGBA)
    vec4 light_direction; // Directional light direction + intensity (xyz = direction, w = intensity)
    vec4 light_color; // Light color (RGB) + ambient intensity (A)
} pushConstants;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    // Extract lighting parameters
    vec3 lightDir = normalize(pushConstants.light_direction.xyz);
    float lightIntensity = pushConstants.light_direction.w;
    vec3 lightColor = pushConstants.light_color.rgb;
    float ambientIntensity = pushConstants.light_color.a;
    
    // Calculate diffuse lighting
    vec3 normal = normalize(fragNormal);
    // Light direction should point FROM the light TO the surface
    float diff = max(dot(normal, -lightDir), 0.0);
    
    // Use lower ambient lighting to make the lighting effect more visible
    vec3 ambient = ambientIntensity * 0.4 * lightColor; // Reduce ambient by 60%
    vec3 diffuse = diff * lightIntensity * lightColor;
    vec3 lighting = ambient + diffuse;
    
    // Clamp lighting to reasonable range
    lighting = clamp(lighting, 0.0, 2.0);
    
    // Apply lighting to material color
    vec3 color = pushConstants.material_color.rgb * lighting;
    fragColor = vec4(color, pushConstants.material_color.a);
}