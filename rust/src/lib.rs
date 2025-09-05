mod post_effect;

use godot::{
    classes::{
        Engine, Image, RdSamplerState, RenderingServer,
        image::Format,
        rendering_device::{SamplerBorderColor, SamplerFilter, SamplerRepeatMode},
    },
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Object, tool)]
pub struct GlobalRidsSingleton {
    base: Base<Object>,
    pub glow_downsample_sampler: Rid,
    pub default_sampler: Rid,
    pub default_sampler_mipmaps: Rid,
    pub default_texture_white: Rid,
    pub default_texture_black: Rid,
    pub default_texture_white_rs: Rid,
    pub default_texture_black_rs: Rid,
}

#[godot_api]
impl IObject for GlobalRidsSingleton {
    fn init(base: Base<Object>) -> Self {
        let mut rs = RenderingServer::singleton();
        let mut rd = rs.get_rendering_device().unwrap();

        let glow_downsample_sampler: Rid = {
            let mut state = RdSamplerState::new_gd();
            state.set_mag_filter(SamplerFilter::LINEAR);
            state.set_min_filter(SamplerFilter::LINEAR);
            state.set_repeat_u(SamplerRepeatMode::CLAMP_TO_BORDER);
            state.set_repeat_v(SamplerRepeatMode::CLAMP_TO_BORDER);
            state.set_border_color(SamplerBorderColor::FLOAT_TRANSPARENT_BLACK);
            rd.sampler_create(&state)
        };

        let default_sampler: Rid = {
            let mut state = RdSamplerState::new_gd();
            state.set_mag_filter(SamplerFilter::LINEAR);
            state.set_min_filter(SamplerFilter::LINEAR);
            state.set_max_lod(0.0);
            state.set_repeat_u(SamplerRepeatMode::CLAMP_TO_EDGE);
            state.set_repeat_v(SamplerRepeatMode::CLAMP_TO_EDGE);
            rd.sampler_create(&state)
        };

        let default_sampler_mipmaps: Rid = {
            let mut state = RdSamplerState::new_gd();
            state.set_mag_filter(SamplerFilter::LINEAR);
            state.set_min_filter(SamplerFilter::LINEAR);
            state.set_mip_filter(SamplerFilter::LINEAR);
            state.set_repeat_u(SamplerRepeatMode::CLAMP_TO_EDGE);
            state.set_repeat_v(SamplerRepeatMode::CLAMP_TO_EDGE);
            rd.sampler_create(&state)
        };

        let default_texture_white_rs: Rid = {
            let mut image = Image::create_empty(16, 16, false, Format::RGBA8).unwrap();
            image.fill(Color::WHITE);
            rs.texture_2d_create(&image)
        };

        let default_texture_white: Rid = rs.texture_get_rd_texture(default_texture_white_rs);

        let default_texture_black_rs: Rid = {
            let mut image = Image::create_empty(16, 16, false, Format::RGBA8).unwrap();
            image.fill(Color::BLACK);
            rs.texture_2d_create(&image)
        };

        let default_texture_black: Rid = rs.texture_get_rd_texture(default_texture_black_rs);

        Self {
            base,
            glow_downsample_sampler,
            default_sampler,
            default_sampler_mipmaps,
            default_texture_white,
            default_texture_black,
            default_texture_white_rs,
            default_texture_black_rs,
        }
    }
}

impl Drop for GlobalRidsSingleton {
    fn drop(&mut self) {
        let mut rs = RenderingServer::singleton();
        rs.free_rid(self.default_texture_white_rs);
        rs.free_rid(self.default_texture_black_rs);

        let mut rd = rs.get_rendering_device().unwrap();
        rd.free_rid(self.glow_downsample_sampler);
        rd.free_rid(self.default_sampler);
        rd.free_rid(self.default_sampler_mipmaps);
    }
}

struct MyExtension;
#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {
    fn on_level_init(level: InitLevel) {
        if level == InitLevel::Scene {}
    }

    fn on_level_deinit(level: InitLevel) {
        if level == InitLevel::Scene {
            let mut engine = Engine::singleton();

            let singleton_name = &GlobalRidsSingleton::class_name().to_string_name();
            let my_singleton = engine.get_singleton(singleton_name).unwrap();
            engine.unregister_singleton(singleton_name);
            my_singleton.free();
        }
    }
}
