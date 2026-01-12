#version 450

// ============================================================================
// PBR Fragment Shader
// Physically-based rendering with multi-light support
// Supports: directional, point, and spot lights
// Features: normal mapping, metallic/roughness workflow, emission, opacity
// ============================================================================

// Include shared definitions
#include "../../common/utils.glsl"
#include "../../common/material.glsl"
#include "../../common/lighting.glsl"
#include "./brdf.glsl"
#include "./pbr_core.glsl"

// Input from vertex shader
layout(location = 0) in vec3 fragPosition;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragTexCoord;
layout(location = 3) in vec3 fragCameraPosition;
layout(location = 4) in vec4 fragInstanceMaterialColor;
layout(location = 5) in flat uint fragInstanceMaterialIndex;
layout(location = 6) in vec4 fragInstanceEmission;
layout(location = 7) in flat uvec4 fragTextureFlags;
layout(location = 8) in vec3 fragTangent;

// Output color
layout(location = 0) out vec4 fragColor;

void main() {
    // ========== Material Sampling ==========
    
    // Base color from material UBO and instance data
    vec3 baseColor = material.base_color.rgb * fragInstanceMaterialColor.rgb;
    float alpha = material.base_color.a * fragInstanceMaterialColor.a;
    
    // Apply base color texture if enabled
    if (fragTextureFlags.x != 0u) {
        vec4 textureColor = texture(baseColorTexture, fragTexCoord);
        baseColor *= textureColor.rgb;
        alpha *= textureColor.a;
    }
    
    // Sample PBR material properties
    float metallic = material.metallic_roughness_ao_normal.x;
    float roughness = material.metallic_roughness_ao_normal.y;
    float ao = material.metallic_roughness_ao_normal.z;
    
    if (material.texture_flags.z != 0u) {
        vec3 metallicRoughness = texture(metallicRoughnessTexture, fragTexCoord).rgb;
        metallic *= metallicRoughness.b;  // Blue channel
        roughness *= metallicRoughness.g; // Green channel
    }
    
    if (material.texture_flags.w != 0u) {
        ao *= texture(aoTexture, fragTexCoord).r;
    }
    
    // ========== Normal Mapping ==========
    
    vec3 normal = normalize(fragNormal);
    if (fragTextureFlags.y != 0u) {
        mat3 TBN = buildTBN(fragNormal, fragTangent);
        normal = sampleNormalMap(normalTexture, fragTexCoord, TBN);
    }
    
    // ========== Lighting Calculation ==========
    
    vec3 viewDir = normalize(fragCameraPosition - fragPosition);
    
    // Start with ambient lighting
    vec3 ambient = lighting.ambient_color.rgb * lighting.ambient_color.a * baseColor * ao;
    vec3 lighting_result = ambient;
    
    // Process directional lights
    for (uint i = 0u; i < lighting.directional_light_count && i < 4u; i++) {
        vec3 lightDir = normalize(-lighting.directional_lights[i].direction.xyz);
        vec3 lightColor = lighting.directional_lights[i].color.rgb;
        float lightIntensity = lighting.directional_lights[i].direction.w;
        
        lighting_result += calculatePBR(
            baseColor, metallic, roughness, normal,
            lightDir, viewDir, lightColor * lightIntensity
        );
    }
    
    // Process point lights
    for (uint i = 0u; i < lighting.point_light_count && i < 64u; i++) {
        vec3 lightPos = lighting.point_lights[i].position.xyz;
        float lightRange = lighting.point_lights[i].position.w;
        vec3 lightColor = lighting.point_lights[i].color.rgb;
        float lightIntensity = lighting.point_lights[i].color.w;
        vec3 attenuation = lighting.point_lights[i].attenuation.xyz;
        
        float atten = calculatePointLightAttenuation(
            lightPos, lightRange, fragPosition, attenuation
        );
        
        if (atten > 0.0) {
            vec3 lightDir = normalize(lightPos - fragPosition);
            vec3 attenuatedColor = lightColor * lightIntensity * atten;
            
            lighting_result += calculatePBR(
                baseColor, metallic, roughness, normal,
                lightDir, viewDir, attenuatedColor
            );
        }
    }
    
    // Process spot lights
    for (uint i = 0u; i < lighting.spot_light_count && i < 4u; i++) {
        vec3 lightPos = lighting.spot_lights[i].position.xyz;
        float lightRange = lighting.spot_lights[i].position.w;
        vec3 spotDir = normalize(lighting.spot_lights[i].direction.xyz);
        float lightIntensity = lighting.spot_lights[i].direction.w;
        vec3 lightColor = lighting.spot_lights[i].color.rgb;
        float innerCone = lighting.spot_lights[i].cone_angles.x;
        float outerCone = lighting.spot_lights[i].cone_angles.y;
        
        vec3 toLight = lightPos - fragPosition;
        float distance = length(toLight);
        
        if (distance <= lightRange) {
            vec3 lightDir = normalize(toLight);
            float intensity = calculateSpotLightIntensity(
                lightDir, spotDir, innerCone, outerCone
            );
            
            if (intensity > 0.0) {
                vec3 attenuatedColor = lightColor * lightIntensity * intensity;
                
                lighting_result += calculatePBR(
                    baseColor, metallic, roughness, normal,
                    lightDir, viewDir, attenuatedColor
                );
            }
        }
    }
    
    // ========== Emission ==========
    
    vec3 emissionColor = fragInstanceEmission.rgb;
    float emissionStrength = fragInstanceEmission.a;
    
    // Sample emission texture if enabled
    if (material.additional_params.x != 0.0) {
        vec4 emissionTex = texture(emissionTexture, fragTexCoord);
        emissionColor *= emissionTex.rgb * emissionTex.a;
    }
    
    lighting_result += emissionColor * emissionStrength;
    
    // ========== Opacity ==========
    
    float finalAlpha = alpha;
    if (material.additional_params.y != 0.0) {
        finalAlpha *= texture(opacityTexture, fragTexCoord).r;
    }
    
    // Alpha cutoff for transparency
    if (finalAlpha < 0.1) {
        discard;
    }
    
    // ========== Output ==========
    
    fragColor = vec4(lighting_result, finalAlpha);
}
