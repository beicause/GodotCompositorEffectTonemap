use godot::{
    classes::{
        Engine, FramebufferCacheRd, RdPipelineColorBlendState, RdPipelineColorBlendStateAttachment,
        RdPipelineDepthStencilState, RdPipelineMultisampleState, RdPipelineRasterizationState,
        RdPipelineSpecializationConstant, RdShaderFile, RdUniform, RenderingDevice,
        RenderingServer, ResourceLoader, UniformSetCacheRd,
        rendering_device::{RenderPrimitive, UniformType},
    },
    meta::AsArg,
    prelude::*,
};
use zerocopy::FromBytes;

use crate::GlobalRidsSingleton;

pub const TEX_COPY_SHADER_PATH: &str = "uid://bky734u2m1ik4";
pub const DOWNSAMPLER_SHADER_PATH: &str = "uid://dn7kvwu3pc8ht";
pub const UPSAMPLE_SHADER_PATH: &str = "uid://d20ptrfi77euk";
pub const TONEMAPPER_SHADER_PATH: &str = "uid://dch7mum06agob";

pub const SC_TONEMAP_TYPE_INDEX: u8 = 2;
pub const SC_GLOW_MODE_INDEX: u8 = 9;
pub const SC_MAX: u8 = 11;

pub struct Renderer {
    pub rd: Gd<RenderingDevice>,
    pub shader: Rid,
    pub pipeline: Rid,
    pub framebuffer: Rid,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if self.framebuffer.is_valid() && self.rd.framebuffer_is_valid(self.framebuffer) {
            self.rd.free_rid(self.framebuffer);
        }
        if self.shader.is_valid() {
            self.rd.free_rid(self.shader);
        }
    }
}

impl Renderer {
    pub fn from_shader_file(shader_file: &Gd<RdShaderFile>) -> Self {
        let mut rd = RenderingServer::singleton().get_rendering_device().unwrap();
        let spirv = shader_file.get_spirv().unwrap();
        let shader = rd.shader_create_from_spirv(&spirv);
        Self {
            rd,
            shader,
            pipeline: Rid::Invalid,
            framebuffer: Rid::Invalid,
        }
    }

    pub fn from_shader_file_path(path: impl AsArg<GString>) -> Self {
        Self::from_shader_file(&ResourceLoader::singleton().load(path).unwrap().cast())
    }

    pub fn setup_pipeline_texure(
        &mut self,
        dst_tex: Rid,
        scs: &Array<Gd<RdPipelineSpecializationConstant>>,
    ) {
        let fb = FramebufferCacheRd::get_cache_multipass(
            &Array::<Rid>::from(&[dst_tex]),
            &Array::new(),
            1,
        );
        let fb_fmt = self.rd.framebuffer_get_format(fb);
        let mut blend_state = RdPipelineColorBlendState::new_gd();
        blend_state.set_attachments(&Array::from(&[
            RdPipelineColorBlendStateAttachment::new_gd(),
        ]));
        let pipeline = self
            .rd
            .render_pipeline_create_ex(
                self.shader,
                fb_fmt,
                RenderingDevice::INVALID_ID.into(),
                RenderPrimitive::TRIANGLES,
                &RdPipelineRasterizationState::new_gd(),
                &RdPipelineMultisampleState::new_gd(),
                &RdPipelineDepthStencilState::new_gd(),
                &blend_state,
            )
            .specialization_constants(scs)
            .done();
        if self.pipeline.is_valid() && self.rd.render_pipeline_is_valid(self.pipeline) {
            self.rd.free_rid(self.pipeline);
        }
        self.pipeline = pipeline;
        self.framebuffer = fb;
    }

    pub fn setup_pipeline_framebuffer(
        &mut self,
        fb: Rid,
        scs: &Array<Gd<RdPipelineSpecializationConstant>>,
    ) {
        let fb_fmt = self.rd.framebuffer_get_format(fb);
        let mut blend_state = RdPipelineColorBlendState::new_gd();
        blend_state.set_attachments(&Array::from(&[
            RdPipelineColorBlendStateAttachment::new_gd(),
        ]));
        let pipeline = self
            .rd
            .render_pipeline_create_ex(
                self.shader,
                fb_fmt,
                RenderingDevice::INVALID_ID.into(),
                RenderPrimitive::TRIANGLES,
                &RdPipelineRasterizationState::new_gd(),
                &RdPipelineMultisampleState::new_gd(),
                &RdPipelineDepthStencilState::new_gd(),
                &blend_state,
            )
            .specialization_constants(scs)
            .done();
        if self.pipeline.is_valid() && self.rd.render_pipeline_is_valid(self.pipeline) {
            self.rd.free_rid(self.pipeline);
        }
        self.pipeline = pipeline;
        self.framebuffer = fb;
    }
}

pub struct TexCopy {
    renderer: Renderer,
    scs: Array<Gd<RdPipelineSpecializationConstant>>,
    uniforms: Array<Gd<RdUniform>>,
    sampler: Rid,
}

impl TexCopy {
    pub fn init() -> Self {
        let mut uniforms = Array::new();
        let mut uniform_src_tex = RdUniform::new_gd();
        uniform_src_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_src_tex.set_binding(0);
        uniforms.push(&uniform_src_tex);

        let sampler = Engine::singleton()
            .get_singleton(&GlobalRidsSingleton::class_name().to_string_name())
            .unwrap()
            .cast::<GlobalRidsSingleton>()
            .bind()
            .default_sampler;
        Self {
            renderer: Renderer::from_shader_file_path(TEX_COPY_SHADER_PATH),
            scs: Array::new(),
            uniforms,
            sampler,
        }
    }

    pub fn exec(&mut self, source_rd_texture: Rid, dest_texture: Rid) {
        // Pipeline.
        self.renderer.setup_pipeline_texure(dest_texture, &self.scs);
        let mut uniform_src_tex = self.uniforms.get(0).unwrap();
        uniform_src_tex.clear_ids();
        uniform_src_tex.add_id(self.sampler);
        uniform_src_tex.add_id(source_rd_texture);
        let uniform_set = UniformSetCacheRd::get_cache(self.renderer.shader, 0, &self.uniforms);
        let draw_list = self.renderer.rd.draw_list_begin(self.renderer.framebuffer);
        self.renderer
            .rd
            .draw_list_bind_render_pipeline(draw_list, self.renderer.pipeline);
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set, 0);
        self.renderer
            .rd
            .draw_list_draw_ex(draw_list, false, 1)
            .procedural_vertex_count(3)
            .done();
        self.renderer.rd.draw_list_end();
    }
}

#[derive(
    Debug,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    Default,
)]
#[repr(C)]
struct BlurDownsamplePushConstants {
    source_pixel_size_x: f32, // 04 - 12
    source_pixel_size_y: f32, // 04 - 16
    pad1: f32,
    pad2: f32,
    // Glow.
    glow_strength: f32,      // 04 - 20
    glow_bloom: f32,         // 04 - 24
    glow_hdr_threshold: f32, // 04 - 28
    glow_hdr_scale: f32,     // 04 - 32

    glow_exposure: f32,        // 04 - 36
    glow_luminance_cap: f32,   // 04 - 40
    luminance_multiplier: f32, // 04 - 44
    pad: f32,                  // 04 - 48
}

pub struct BlurDownsample {
    renderer: Renderer,
    scs: Array<Gd<RdPipelineSpecializationConstant>>,
    ubo: PackedArray<u8>,
    uniforms: Array<Gd<RdUniform>>,
    sampler: Rid,
}

impl BlurDownsample {
    pub fn init() -> Self {
        let mut sc = RdPipelineSpecializationConstant::new_gd();
        sc.set_constant_id(0);
        sc.set_value(&false.to_variant());
        let ubo_bytes: [u8; std::mem::size_of::<BlurDownsamplePushConstants>()] =
            zerocopy::transmute!(BlurDownsamplePushConstants::default());
        let ubo = PackedArray::<u8>::from(&ubo_bytes);
        let mut uniforms = Array::new();
        let mut uniform_src_tex = RdUniform::new_gd();
        uniform_src_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_src_tex.set_binding(0);
        uniforms.push(&uniform_src_tex);

        let sampler = Engine::singleton()
            .get_singleton(&GlobalRidsSingleton::class_name().to_string_name())
            .unwrap()
            .cast::<GlobalRidsSingleton>()
            .bind()
            .glow_downsample_sampler;

        Self {
            renderer: Renderer::from_shader_file_path(DOWNSAMPLER_SHADER_PATH),
            scs: Array::from(&[sc]),
            ubo,
            uniforms,
            sampler,
        }
    }

    pub fn exec(
        &mut self,
        source_rd_texture: Rid,
        dest_texture: Rid,
        luminance_multiplier: f32,
        size: Vector2i,
        strength: f32,
        first_pass: bool,
        luminance_cap: f32,
        exposure: f32,
        bloom: f32,
        hdr_bleed_threshold: f32,
        hdr_bleed_scale: f32,
    ) {
        // Specialization constant.
        self.scs.get(0).unwrap().set_value(&first_pass.to_variant());
        // Pipeline.
        self.renderer.setup_pipeline_texure(dest_texture, &self.scs);
        // UBO.
        let ubo = self.ubo.as_mut_slice();
        let ubo_mut = BlurDownsamplePushConstants::mut_from_bytes(ubo).unwrap();
        ubo_mut.source_pixel_size_x = 1.0 / size.x as f32;
        ubo_mut.source_pixel_size_y = 1.0 / size.y as f32;
        ubo_mut.glow_strength = strength;
        ubo_mut.glow_bloom = bloom;
        ubo_mut.glow_hdr_threshold = hdr_bleed_threshold;
        ubo_mut.glow_hdr_scale = hdr_bleed_scale;
        ubo_mut.glow_exposure = exposure;
        ubo_mut.glow_luminance_cap = luminance_cap;
        ubo_mut.luminance_multiplier = luminance_multiplier;
        let mut uniform_src_tex = self.uniforms.get(0).unwrap();
        uniform_src_tex.clear_ids();
        uniform_src_tex.add_id(self.sampler);
        uniform_src_tex.add_id(source_rd_texture);
        let uniform_set = UniformSetCacheRd::get_cache(self.renderer.shader, 0, &self.uniforms);
        let draw_list = self.renderer.rd.draw_list_begin(self.renderer.framebuffer);
        self.renderer
            .rd
            .draw_list_bind_render_pipeline(draw_list, self.renderer.pipeline);
        self.renderer.rd.draw_list_set_push_constant(
            draw_list,
            &self.ubo,
            self.ubo.len().try_into().unwrap(),
        );
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set, 0);
        self.renderer
            .rd
            .draw_list_draw_ex(draw_list, false, 1)
            .procedural_vertex_count(3)
            .done();
        self.renderer.rd.draw_list_end();
    }
}

#[derive(
    Debug,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    Default,
)]
#[repr(C)]
struct BlurUpsamplePushConstants {
    dest_pixel_size_x: f32,   // 04 - 04
    dest_pixel_size_y: f32,   // 04 - 08
    source_pixel_size_x: f32, // 04 - 12
    source_pixel_size_y: f32, // 04 - 16
    // Glow.
    glow_level: f32,    // 04 - 20
    glow_strength: f32, // 04 - 24
    pad1: f32,          // 04 - 28
    pad2: f32,          // 04 - 32
}

pub struct BlurUpsample {
    renderer: Renderer,
    scs: Array<Gd<RdPipelineSpecializationConstant>>,
    ubo: PackedArray<u8>,
    uniforms_src: Array<Gd<RdUniform>>,
    uniforms_blend: Array<Gd<RdUniform>>,
    sampler: Rid,
}

impl BlurUpsample {
    pub fn init() -> Self {
        let mut sc = RdPipelineSpecializationConstant::new_gd();
        sc.set_constant_id(0);
        sc.set_value(&false.to_variant());
        let ubo_bytes: [u8; std::mem::size_of::<BlurUpsamplePushConstants>()] =
            zerocopy::transmute!(BlurUpsamplePushConstants::default());
        let ubo = PackedArray::<u8>::from(&ubo_bytes);

        let mut uniforms_src = Array::new();
        let mut uniform_src_tex = RdUniform::new_gd();
        uniform_src_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_src_tex.set_binding(0);
        uniforms_src.push(&uniform_src_tex);

        let mut uniforms_blend = Array::new();
        let mut uniform_blend_tex = RdUniform::new_gd();
        uniform_blend_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_blend_tex.set_binding(0);
        uniforms_blend.push(&uniform_blend_tex);

        let sampler = Engine::singleton()
            .get_singleton(&GlobalRidsSingleton::class_name().to_string_name())
            .unwrap()
            .cast::<GlobalRidsSingleton>()
            .bind()
            .default_sampler;
        Self {
            renderer: Renderer::from_shader_file_path(UPSAMPLE_SHADER_PATH),
            scs: Array::from(&[sc]),
            ubo,
            uniforms_src,
            uniforms_blend,
            sampler,
        }
    }
    pub fn exec(
        &mut self,
        source_rd_texture: Rid,
        dest_texture: Rid,
        blend_texture: Rid,
        source_size: Vector2i,
        dest_size: Vector2i,
        level: f32,
        base_strength: f32,
    ) {
        // Specialization constant.
        self.scs
            .get(0)
            .unwrap()
            .set_value(&(level > 0.01).to_variant());
        // Pipeline.
        self.renderer.setup_pipeline_texure(dest_texture, &self.scs);
        // UBO.
        let ubo = self.ubo.as_mut_slice();
        let ubo_mut = BlurUpsamplePushConstants::mut_from_bytes(ubo).unwrap();
        ubo_mut.source_pixel_size_x = 1.0 / source_size.x as f32;
        ubo_mut.source_pixel_size_y = 1.0 / source_size.y as f32;
        ubo_mut.dest_pixel_size_x = 1.0 / dest_size.x as f32;
        ubo_mut.dest_pixel_size_y = 1.0 / dest_size.y as f32;
        ubo_mut.glow_level = level * 0.5;
        ubo_mut.glow_strength = base_strength;

        let mut uniform_src_tex = self.uniforms_src.get(0).unwrap();
        uniform_src_tex.clear_ids();
        uniform_src_tex.add_id(self.sampler);
        uniform_src_tex.add_id(source_rd_texture);

        let mut uniform_blend_tex = self.uniforms_blend.get(0).unwrap();
        uniform_blend_tex.clear_ids();
        uniform_blend_tex.add_id(self.sampler);
        uniform_blend_tex.add_id(blend_texture);

        let uniform_set0 =
            UniformSetCacheRd::get_cache(self.renderer.shader, 0, &self.uniforms_src);
        let uniform_set1 =
            UniformSetCacheRd::get_cache(self.renderer.shader, 1, &self.uniforms_blend);

        let draw_list = self.renderer.rd.draw_list_begin(self.renderer.framebuffer);
        self.renderer
            .rd
            .draw_list_bind_render_pipeline(draw_list, self.renderer.pipeline);
        self.renderer.rd.draw_list_set_push_constant(
            draw_list,
            &self.ubo,
            self.ubo.len().try_into().unwrap(),
        );
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set0, 0);
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set1, 1);
        self.renderer
            .rd
            .draw_list_draw_ex(draw_list, false, 1)
            .procedural_vertex_count(3)
            .done();
        self.renderer.rd.draw_list_end();
    }
}

#[derive(
    Debug,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    Default,
)]
#[repr(C)]
struct ToneMapperPushConstants {
    dest_pixel_size_x: f32, // 04 - 20
    dest_pixel_size_y: f32, // 04 - 24
    glow_pixel_size_x: f32, // 04 - 28
    glow_pixel_size_y: f32, // 04 - 32

    glow_intensity: f32,    // 04 - 36
    glow_map_strength: f32, // 04 - 40
    exposure: f32,          // 04 - 44
    white: f32,             // 04 - 48
}

#[derive(GodotConvert, Var, Export, Clone, Copy)]
#[godot(via = i64)]
pub enum ToneMapperType {
    Linear,
    Reinhard,
    Filmic,
    Aces,
    Agx,
    Gt,
    Lottes,
}

#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Eq)]
#[godot(via = i64)]
pub enum GlowMode {
    Add,
    Replace,
    Mix,
}

pub struct ToneMapSettings {
    pub glow_tex_size: Vector2i,
    pub glow_tex: Rid,
    pub use_glow_map: bool,
    pub glow_map_tex: Rid,
    pub glow_intensity: f32,
    pub glow_map_strength: f32,
    pub exposure: f32,
    pub white: f32,
    pub use_fxaa: bool,
    pub tonemap_type: ToneMapperType,
    pub glow_mode: GlowMode,
}

pub struct ToneMapper {
    renderer: Renderer,
    scs: Array<Gd<RdPipelineSpecializationConstant>>,
    ubo: PackedArray<u8>,
    uniforms_src: Array<Gd<RdUniform>>,
    uniforms_glow: Array<Gd<RdUniform>>,
    sampler: Rid,
    sampler_mipmaps: Rid,
    default_tex_white: Rid,
}

impl ToneMapper {
    pub fn init() -> Self {
        let mut scs = Array::new();
        for i in 0..=SC_MAX {
            let mut sc = RdPipelineSpecializationConstant::new_gd();
            sc.set_constant_id(i.into());
            sc.set_value(&false.to_variant());
            scs.push(&sc);
        }

        let ubo_bytes: [u8; std::mem::size_of::<ToneMapperPushConstants>()] =
            zerocopy::transmute!(ToneMapperPushConstants::default());
        let ubo = PackedArray::<u8>::from(&ubo_bytes);

        let mut uniforms_src = Array::new();
        let mut uniform_src_tex = RdUniform::new_gd();
        uniform_src_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_src_tex.set_binding(0);
        uniforms_src.push(&uniform_src_tex);

        let mut uniforms_glow = Array::new();
        let mut uniform_glow_tex = RdUniform::new_gd();
        uniform_glow_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_glow_tex.set_binding(0);
        let mut uniform_glow_map_tex = RdUniform::new_gd();
        uniform_glow_map_tex.set_uniform_type(UniformType::SAMPLER_WITH_TEXTURE);
        uniform_glow_map_tex.set_binding(1);
        uniforms_glow.push(&uniform_glow_tex);
        uniforms_glow.push(&uniform_glow_map_tex);

        let singleton = Engine::singleton()
            .get_singleton(&GlobalRidsSingleton::class_name().to_string_name())
            .unwrap()
            .cast::<GlobalRidsSingleton>();
        let sampler = singleton.bind().default_sampler;
        let sampler_mipmaps = singleton.bind().default_sampler_mipmaps;
        let default_tex_white = singleton.bind().default_texture_white;

        Self {
            renderer: Renderer::from_shader_file_path(TONEMAPPER_SHADER_PATH),
            scs,
            ubo,
            uniforms_src,
            uniforms_glow,
            sampler,
            sampler_mipmaps,
            default_tex_white,
        }
    }
    pub fn exec(
        &mut self,
        source_rd_texture: Rid,
        dest_framebuffer: Rid,
        dest_size: Vector2i,
        settings: ToneMapSettings,
    ) {
        // Specialization constant.
        let fv = false.to_variant();
        let tv = true.to_variant();
        for i in 0..10 {
            self.scs.get(i).unwrap().set_value(&fv);
        }
        self.scs
            .get(0)
            .unwrap()
            .set_value(&settings.use_glow_map.to_variant());
        self.scs
            .get(1)
            .unwrap()
            .set_value(&settings.use_fxaa.to_variant());
        self.scs
            .get(settings.tonemap_type as usize + Into::<usize>::into(SC_TONEMAP_TYPE_INDEX))
            .unwrap()
            .set_value(&tv);
        self.scs
            .get(settings.glow_mode as usize + Into::<usize>::into(SC_GLOW_MODE_INDEX))
            .unwrap()
            .set_value(&tv);
        // use_glow_map
        // use_fxaa
        // tonemapper_linear
        // tonemapper_reinhard
        // tonemapper_filmic
        // tonemapper_aces
        // tonemapper_agx
        // glow_mode_add
        // glow_mode_replace
        // glow_mode_mix

        // Pipeline.
        self.renderer
            .setup_pipeline_framebuffer(dest_framebuffer, &self.scs);
        // UBO.
        let ubo = self.ubo.as_mut_slice();
        let ubo_mut = ToneMapperPushConstants::mut_from_bytes(ubo).unwrap();
        ubo_mut.dest_pixel_size_x = 1.0 / dest_size.x as f32;
        ubo_mut.dest_pixel_size_y = 1.0 / dest_size.y as f32;
        ubo_mut.glow_pixel_size_x = 1.0 / settings.glow_tex_size.x as f32;
        ubo_mut.glow_pixel_size_y = 1.0 / settings.glow_tex_size.y as f32;
        ubo_mut.glow_intensity = settings.glow_intensity;
        ubo_mut.glow_map_strength = settings.glow_map_strength;
        ubo_mut.exposure = settings.exposure;
        ubo_mut.white = settings.white;

        let mut uniform_src_tex = self.uniforms_src.get(0).unwrap();
        uniform_src_tex.clear_ids();
        uniform_src_tex.add_id(self.sampler);
        uniform_src_tex.add_id(source_rd_texture);

        let mut uniform_glow_tex = self.uniforms_glow.get(0).unwrap();
        uniform_glow_tex.clear_ids();
        uniform_glow_tex.add_id(self.sampler_mipmaps);
        uniform_glow_tex.add_id(settings.glow_tex);

        let mut uniform_glow_map_tex = self.uniforms_glow.get(1).unwrap();
        uniform_glow_map_tex.clear_ids();
        uniform_glow_map_tex.add_id(self.sampler_mipmaps);
        if settings.glow_map_tex.is_valid() {
            uniform_glow_map_tex.add_id(settings.glow_map_tex);
        } else {
            uniform_glow_map_tex.add_id(self.default_tex_white);
        }

        let uniform_set0 =
            UniformSetCacheRd::get_cache(self.renderer.shader, 0, &self.uniforms_src);
        let uniform_set1 =
            UniformSetCacheRd::get_cache(self.renderer.shader, 1, &self.uniforms_glow);

        let draw_list = self.renderer.rd.draw_list_begin(self.renderer.framebuffer);
        self.renderer
            .rd
            .draw_list_bind_render_pipeline(draw_list, self.renderer.pipeline);
        self.renderer.rd.draw_list_set_push_constant(
            draw_list,
            &self.ubo,
            self.ubo.len().try_into().unwrap(),
        );
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set0, 0);
        self.renderer
            .rd
            .draw_list_bind_uniform_set(draw_list, uniform_set1, 1);
        self.renderer
            .rd
            .draw_list_draw_ex(draw_list, false, 1)
            .procedural_vertex_count(3)
            .done();
        self.renderer.rd.draw_list_end();
    }
}
