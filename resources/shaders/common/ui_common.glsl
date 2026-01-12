// UI Common definitions
// Shared constants and structures for UI rendering

// Push constants structure for UI color
// Used by both simple UI and text rendering
struct UIColorPushConstants {
    vec4 color;  // RGBA color
};

// UI coordinate space utilities
// UI coordinates are in normalized device coordinates (-1 to 1)
vec4 ndcToClipSpace(vec2 ndcPosition) {
    return vec4(ndcPosition, 0.0, 1.0);
}
