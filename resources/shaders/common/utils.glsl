// Common shader utility functions

// Constants
const float PI = 3.14159265359;

// Safe division to prevent divide-by-zero
float safeDivide(float numerator, float denominator, float epsilon) {
    return numerator / max(denominator, epsilon);
}

// Reconstruct matrix from vec4 columns
mat4 mat4FromColumns(vec4 c0, vec4 c1, vec4 c2, vec4 c3) {
    return mat4(c0, c1, c2, c3);
}

// Reconstruct mat3 from first 3 components of vec4s
mat3 mat3FromVec4Columns(vec4 c0, vec4 c1, vec4 c2) {
    return mat3(c0.xyz, c1.xyz, c2.xyz);
}

// Build orthonormal TBN matrix for normal mapping
mat3 buildTBN(vec3 normal, vec3 tangent) {
    vec3 N = normalize(normal);
    vec3 T = normalize(tangent);
    
    // Gram-Schmidt orthogonalization
    T = normalize(T - dot(T, N) * N);
    
    // Calculate bitangent
    vec3 B = cross(N, T);
    
    return mat3(T, B, N);
}

// Sample and transform normal from normal map
vec3 sampleNormalMap(sampler2D normalMap, vec2 texCoord, mat3 TBN) {
    // Sample and convert from [0,1] to [-1,1]
    vec3 normalMapSample = texture(normalMap, texCoord).rgb;
    normalMapSample = normalMapSample * 2.0 - 1.0;
    
    // Transform from tangent space to world space
    return normalize(TBN * normalMapSample);
}
