pub mod copy;

use std::sync::LazyLock;

use godot::{
    classes::{
        CompositorEffect, Engine, FramebufferCacheRd, ICompositorEffect, RenderData,
        RenderSceneBuffersRd, RenderingDevice, RenderingServer, Texture2D,
        compositor_effect::EffectCallbackType,
        rendering_device::{TextureSamples, TextureUsageBits},
    },
    prelude::*,
};

use crate::{
    GlobalRidsSingleton,
    post_effect::copy::{
        BlurDownsample, BlurUpsample, GlowMode, TexCopy, ToneMapper, ToneMapperType,
    },
};

static RB_SCOPE_BUFFERS: LazyLock<StringName> =
    LazyLock::new(|| StringName::from(c"my_render_buffers"));
static RB_TEX_BLUR_0: LazyLock<StringName> = LazyLock::new(|| StringName::from(c"blur_0"));
static RB_TEX_BLUR_1: LazyLock<StringName> = LazyLock::new(|| StringName::from(c"blur_1"));

#[derive(GodotClass)]
#[class(base=CompositorEffect,tool)]
pub struct PostEffectToneMap {
    base: Base<CompositorEffect>,
    rd: Gd<RenderingDevice>,
    downsample: BlurDownsample,
    upsample: BlurUpsample,
    tonemapper: ToneMapper,
    copy: TexCopy,
    global_rids_singleton: Gd<GlobalRidsSingleton>,

    #[export]
    use_fxaa: bool,
    #[export]
    glow_levels: PackedArray<f32>,
    #[export]
    glow_intensity: f32,
    #[export]
    glow_strength: f32,
    #[export]
    glow_mix: f32,
    #[export]
    glow_bloom: f32,
    #[export]
    glow_blend_mode: GlowMode,
    #[export]
    glow_hdr_bleed_threshold: f32,
    #[export]
    glow_hdr_bleed_scale: f32,
    #[export]
    glow_hdr_luminance_cap: f32,
    #[export]
    glow_map_strength: f32,
    #[export]
    glow_map: Option<Gd<Texture2D>>,
    #[export]
    exposure: f32,
    #[export]
    white: f32,
    #[export]
    tonemap_type: ToneMapperType,
}

#[godot_api]
impl ICompositorEffect for PostEffectToneMap {
    fn init(base: Base<CompositorEffect>) -> Self {
        let engine = Engine::singleton();
        let name = GlobalRidsSingleton::class_name().to_string_name();
        if !engine.has_singleton(&name) {
            // Workaround for https://github.com/godotengine/godot-cpp/issues/1180.
            // Should be replaced by GDExtensionMainLoopStartupCallback in Godot 4.5.
            Engine::singleton().register_singleton(
                &GlobalRidsSingleton::class_name().to_string_name(),
                &GlobalRidsSingleton::new_alloc(),
            );
        }

        let rd = RenderingServer::singleton().get_rendering_device().unwrap();
        let glow_levels = [1.0, 1.0, 1.0];
        let glow_intensity: f32 = 0.8;
        let glow_strength: f32 = 1.0;
        let glow_mix: f32 = 0.05;
        let glow_bloom: f32 = 0.0;
        let glow_blend_mode = GlowMode::Add;
        let glow_hdr_bleed_threshold: f32 = 0.2;
        let glow_hdr_bleed_scale: f32 = 2.0;
        let glow_hdr_luminance_cap: f32 = 12.0;
        let glow_map_strength: f32 = 0.8;
        let glow_map: Option<Gd<Texture2D>> = None;
        let exposure: f32 = 1.0;
        let white: f32 = 2.0;
        let tonemap_type = ToneMapperType::Reinhard;
        Self {
            base,
            rd,
            downsample: BlurDownsample::init(),
            upsample: BlurUpsample::init(),
            tonemapper: ToneMapper::init(),
            copy: TexCopy::init(),
            global_rids_singleton: Engine::singleton()
                .get_singleton(&GlobalRidsSingleton::class_name().to_string_name())
                .unwrap()
                .cast::<GlobalRidsSingleton>(),
            glow_levels: PackedArray::from(&glow_levels),
            use_fxaa: false,
            glow_intensity,
            glow_strength,
            glow_mix,
            glow_bloom,
            glow_blend_mode,
            glow_hdr_bleed_threshold,
            glow_hdr_bleed_scale,
            glow_hdr_luminance_cap,
            glow_map_strength,
            glow_map,
            exposure,
            white,
            tonemap_type,
        }
    }

    fn render_callback(&mut self, effect_callback_type: i32, render_data: Option<Gd<RenderData>>) {
        if effect_callback_type != EffectCallbackType::POST_TRANSPARENT.ord() {
            return;
        }
        let data = render_data.unwrap();
        let mut rb = data
            .get_render_scene_buffers()
            .unwrap()
            .cast::<RenderSceneBuffersRd>();

        if rb.get_view_count() != 1 {
            return;
        }

        let color_tex = rb.get_color_texture();
        let color_fmt = self.rd.texture_get_format(color_tex).unwrap();
        let color_data_fmt = color_fmt.get_format();
        let buffer_size = rb.get_internal_size();
        let scope = &*RB_SCOPE_BUFFERS;
        let blur0 = &*RB_TEX_BLUR_0;
        let blur1 = &*RB_TEX_BLUR_1;
        let _tex_blur0 = rb.create_texture(
            scope,
            blur0,
            color_data_fmt,
            (TextureUsageBits::COLOR_ATTACHMENT_BIT.ord() | TextureUsageBits::SAMPLING_BIT.ord())
                .try_into()
                .unwrap(),
            TextureSamples::SAMPLES_1,
            buffer_size,
            1,
            get_image_required_mipmaps(
                buffer_size.x.try_into().unwrap(),
                buffer_size.y.try_into().unwrap(),
                1,
            ),
            true,
            false,
        );
        let _tex_blur1 = rb.create_texture(
            scope,
            blur1,
            color_data_fmt,
            (TextureUsageBits::COLOR_ATTACHMENT_BIT.ord() | TextureUsageBits::SAMPLING_BIT.ord())
                .try_into()
                .unwrap(),
            TextureSamples::SAMPLES_1,
            Vector2i {
                x: buffer_size.x >> 1,
                y: buffer_size.y >> 1,
            },
            1,
            get_image_required_mipmaps(
                (buffer_size.x >> 1).try_into().unwrap(),
                (buffer_size.y >> 1).try_into().unwrap(),
                1,
            ),
            true,
            false,
        );
        let glow_levels = self.glow_levels.as_slice();
        let mut glow_intensity: f32 = self.glow_intensity;
        let mut glow_map = Rid::Invalid;
        if self.glow_map.is_some() {
            let tex: &Gd<Texture2D> = self.glow_map.as_ref().unwrap();
            let rid = tex.get_rid();
            if rid.is_valid() {
                glow_map = RenderingServer::singleton().texture_get_rd_texture(tex.get_rid());
            }
        }
        if self.glow_blend_mode == GlowMode::Mix {
            glow_intensity = self.glow_mix;
        }

        let mut max_glow_index: i32 = -1;
        let mut min_glow_level: i32 = glow_levels.len().try_into().unwrap();
        for i in 0i32..glow_levels.len().try_into().unwrap() {
            if glow_levels[TryInto::<usize>::try_into(i).unwrap()] > 0.01 {
                max_glow_index = std::cmp::max(max_glow_index, i);
                min_glow_level = std::cmp::min(min_glow_level, i);
            }
        }

        let mut source = color_tex;
        let mut dest = rb.get_texture_slice(scope, blur1, 0, 1, 1, 1);
        let mut source_size = buffer_size;
        let luminance_multiplier = 2.0f32;
        // Downsample.
        self.downsample.exec(
            source,
            dest,
            luminance_multiplier,
            source_size,
            self.glow_strength,
            true,
            self.glow_hdr_luminance_cap,
            self.exposure,
            self.glow_bloom,
            self.glow_hdr_bleed_threshold,
            self.glow_hdr_bleed_scale,
        );
        let mut vp_size;
        for i in 1..max_glow_index + 1 {
            source = dest;
            vp_size = rb.get_texture_slice_size(scope, blur1, i.try_into().unwrap());
            dest = rb.get_texture_slice(scope, blur1, 0, (i + 1).try_into().unwrap(), 1, 1);
            self.downsample.exec(
                source,
                dest,
                luminance_multiplier,
                vp_size,
                self.glow_strength,
                false,
                self.glow_hdr_luminance_cap,
                self.exposure,
                self.glow_bloom,
                self.glow_hdr_bleed_threshold,
                self.glow_hdr_bleed_scale,
            );
        }
        // Upsample.
        if max_glow_index <= 0 {
            source = self.global_rids_singleton.bind().default_texture_black;
            vp_size = rb.get_texture_slice_size(scope, blur0, 2);
            dest = rb.get_texture_slice(scope, blur0, 0, 2, 1, 1);
            let blend_tex = rb.get_texture_slice(scope, blur1, 0, 1, 1, 1);
            source_size = vp_size;
            self.upsample.exec(
                source,
                dest,
                blend_tex,
                source_size,
                vp_size,
                glow_levels[0],
                0.0,
            );
        }
        for i in (0..max_glow_index).rev() {
            source = dest;
            source_size = rb.get_texture_slice_size(scope, blur0, (i + 3).try_into().unwrap());
            vp_size = rb.get_texture_slice_size(scope, blur0, (i + 2).try_into().unwrap());
            dest = rb.get_texture_slice(scope, blur0, 0, (i + 2).try_into().unwrap(), 1, 1);
            let blend_tex =
                rb.get_texture_slice(scope, blur1, 0, (i + 1).try_into().unwrap(), 1, 1);
            self.upsample.exec(
                source,
                dest,
                blend_tex,
                source_size,
                vp_size,
                glow_levels[TryInto::<usize>::try_into(i).unwrap()],
                if i == max_glow_index - 1 {
                    glow_levels[TryInto::<usize>::try_into(i + 1).unwrap()]
                } else {
                    1.0
                },
            );
        }
        let dest_fb =
            FramebufferCacheRd::get_cache_multipass(&Array::from(&[color_tex]), &Array::new(), 1);
        let blur0level0 = rb.get_texture_slice(scope, blur0, 0, 0, 1, 1);
        let blur0level2 = rb.get_texture_slice(scope, blur0, 0, 2, 1, 1);
        self.copy.exec(color_tex, blur0level0);
        self.tonemapper.exec(
            blur0level0,
            dest_fb,
            buffer_size,
            copy::ToneMapSettings {
                glow_tex_size: rb.get_texture_slice_size(scope, blur0, 2),
                glow_tex: blur0level2,
                use_glow_map: glow_map.is_valid(),
                glow_map_tex: glow_map,
                glow_intensity,
                glow_map_strength: self.glow_map_strength,
                exposure: self.exposure,
                white: self.white,
                use_fxaa: self.use_fxaa,
                tonemap_type: self.tonemap_type,
                glow_mode: self.glow_blend_mode,
            },
        );
    }
}

fn get_image_required_mipmaps(width: u32, height: u32, depth: u32) -> u32 {
    let mut w = width;
    let mut h = height;
    let mut d = depth;

    let mut mipmaps = 1;

    loop {
        if w == 1 && h == 1 && d == 1 {
            break;
        }

        w = std::cmp::max(1, w >> 1);
        h = std::cmp::max(1, h >> 1);
        d = std::cmp::max(1, d >> 1);

        mipmaps += 1;
    }

    mipmaps
}
