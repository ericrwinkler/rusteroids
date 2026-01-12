// Core PBR lighting calculation

// Calculate PBR lighting for a single light source
// Returns the combined diffuse + specular contribution
vec3 calculatePBR(
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 normal,
    vec3 lightDir,
    vec3 viewDir,
    vec3 lightColor
) {
    float NdotL = max(dot(normal, lightDir), 0.0);
    float NdotV = max(dot(normal, viewDir), 0.0);
    vec3 halfDir = normalize(lightDir + viewDir);
    float NdotH = max(dot(normal, halfDir), 0.0);
    float VdotH = max(dot(viewDir, halfDir), 0.0);
    
    // Fresnel reflectance at normal incidence
    // For dielectrics use 0.04, for metals use albedo
    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    
    // Cook-Torrance BRDF components
    vec3 F = fresnelSchlick(VdotH, F0);
    float D = distributionGGX(NdotH, roughness);
    float G = geometrySmith(NdotV, NdotL, roughness);
    
    // Specular component
    vec3 numerator = D * G * F;
    float denominator = max(4.0 * NdotV * NdotL, 0.001);
    vec3 specular = numerator / denominator;
    
    // Diffuse component (Lambertian with energy conservation)
    vec3 kS = F; // Specular reflection coefficient
    vec3 kD = vec3(1.0) - kS; // Diffuse reflection coefficient
    kD *= 1.0 - metallic; // Metallic surfaces have no diffuse
    
    vec3 diffuse = kD * albedo / PI;
    
    // Combine diffuse and specular
    return (diffuse + specular) * lightColor * NdotL;
}

// Calculate point light attenuation
float calculatePointLightAttenuation(
    vec3 lightPos,
    float lightRange,
    vec3 fragPos,
    vec3 attenuation
) {
    vec3 lightDir = lightPos - fragPos;
    float distance = length(lightDir);
    
    // Return 0 if beyond range
    if (distance > lightRange) return 0.0;
    
    // Standard inverse square attenuation
    float atten = 1.0 / (
        attenuation.x +
        attenuation.y * distance +
        attenuation.z * distance * distance
    );
    
    // Smooth falloff near range limit
    float rangeFactor = 1.0 - smoothstep(lightRange * 0.7, lightRange, distance);
    return atten * rangeFactor;
}

// Calculate spot light cone intensity
float calculateSpotLightIntensity(
    vec3 lightDir,
    vec3 spotDirection,
    float innerCone,
    float outerCone
) {
    float theta = dot(lightDir, -spotDirection);
    float epsilon = innerCone - outerCone;
    return clamp((theta - outerCone) / epsilon, 0.0, 1.0);
}
