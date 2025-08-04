// src/render/sprite.rs
use crate::util::math::{Vec2, Vec3};
use crate::render::texture::Texture;

#[derive(Debug, Clone)]
pub struct Sprite {
    pub texture_id: usize,
    pub position: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    pub color: [f32; 4],
    pub uv_offset: Vec2,    // For sprite atlases
    pub uv_scale: Vec2,     // For sprite atlases
    pub layer: i32,         // Sorting layer for 2D rendering
}

impl Sprite {
    pub fn new(texture_id: usize) -> Self {
        Self {
            texture_id,
            position: Vec2::zero(),
            size: Vec2::new(1.0, 1.0),
            rotation: 0.0,
            color: [1.0, 1.0, 1.0, 1.0], // White, fully opaque
            uv_offset: Vec2::zero(),
            uv_scale: Vec2::new(1.0, 1.0),
            layer: 0,
        }
    }

    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    // For sprite atlases - specify which part of the texture to use
    pub fn with_uv_rect(mut self, offset: Vec2, scale: Vec2) -> Self {
        self.uv_offset = offset;
        self.uv_scale = scale;
        self
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    pub fn move_by(&mut self, delta: Vec2) {
        self.position = self.position + delta;
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    pub fn rotate_by(&mut self, delta_rotation: f32) {
        self.rotation += delta_rotation;
    }

    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
    }

    pub fn scale_by(&mut self, scale: f32) {
        self.size = self.size * scale;
    }
}

// Animation system for sprites
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    pub name: String,
    pub frames: Vec<SpriteFrame>,
    pub current_frame: usize,
    pub frame_time: f32,
    pub elapsed_time: f32,
    pub looping: bool,
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub struct SpriteFrame {
    pub uv_offset: Vec2,
    pub uv_scale: Vec2,
    pub duration: f32,
}

impl SpriteAnimation {
    pub fn new(name: String, frames: Vec<SpriteFrame>, frame_time: f32) -> Self {
        Self {
            name,
            frames,
            current_frame: 0,
            frame_time,
            elapsed_time: 0.0,
            looping: true,
            finished: false,
        }
    }

    pub fn from_atlas(
        name: String,
        atlas_cols: u32,
        atlas_rows: u32,
        start_frame: u32,
        frame_count: u32,
        frame_time: f32,
    ) -> Self {
        let mut frames = Vec::new();
        let frame_width = 1.0 / atlas_cols as f32;
        let frame_height = 1.0 / atlas_rows as f32;

        for i in 0..frame_count {
            let frame_index = start_frame + i;
            let col = frame_index % atlas_cols;
            let row = frame_index / atlas_cols;
            
            frames.push(SpriteFrame {
                uv_offset: Vec2::new(col as f32 * frame_width, row as f32 * frame_height),
                uv_scale: Vec2::new(frame_width, frame_height),
                duration: frame_time,
            });
        }

        Self::new(name, frames, frame_time)
    }

    pub fn update(&mut self, delta_time: f32) {
        if self.finished && !self.looping {
            return;
        }

        self.elapsed_time += delta_time;

        if let Some(current_frame) = self.frames.get(self.current_frame) {
            if self.elapsed_time >= current_frame.duration {
                self.elapsed_time = 0.0;
                self.current_frame += 1;

                if self.current_frame >= self.frames.len() {
                    if self.looping {
                        self.current_frame = 0;
                    } else {
                        self.current_frame = self.frames.len() - 1;
                        self.finished = true;
                    }
                }
            }
        }
    }

    pub fn get_current_frame(&self) -> Option<&SpriteFrame> {
        self.frames.get(self.current_frame)
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed_time = 0.0;
        self.finished = false;
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

// Component for animated sprites
#[derive(Debug, Clone)]
pub struct AnimatedSprite {
    pub sprite: Sprite,
    pub animations: Vec<SpriteAnimation>,
    pub current_animation: Option<usize>,
}

impl AnimatedSprite {
    pub fn new(sprite: Sprite) -> Self {
        Self {
            sprite,
            animations: Vec::new(),
            current_animation: None,
        }
    }

    pub fn add_animation(&mut self, animation: SpriteAnimation) -> usize {
        let id = self.animations.len();
        self.animations.push(animation);
        id
    }

    pub fn play_animation(&mut self, animation_id: usize) {
        if animation_id < self.animations.len() {
            self.current_animation = Some(animation_id);
            if let Some(anim) = self.animations.get_mut(animation_id) {
                anim.reset();
            }
        }
    }

    pub fn play_animation_by_name(&mut self, name: &str) {
        if let Some(pos) = self.animations.iter().position(|anim| anim.name == name) {
            self.play_animation(pos);
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        if let Some(anim_id) = self.current_animation {
            if let Some(animation) = self.animations.get_mut(anim_id) {
                animation.update(delta_time);
                
                // Update sprite UV coordinates based on current frame
                if let Some(frame) = animation.get_current_frame() {
                    self.sprite.uv_offset = frame.uv_offset;
                    self.sprite.uv_scale = frame.uv_scale;
                }
            }
        }
    }

    pub fn is_animation_finished(&self) -> bool {
        if let Some(anim_id) = self.current_animation {
            if let Some(animation) = self.animations.get(anim_id) {
                return animation.is_finished();
            }
        }
        false
    }
}
