struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct LightUniform {
    dir: vec3<f32>,
    _padding0: f32,
    color: vec3<f32>,
    _padding1: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(0) @binding(1)
var<uniform> light: LightUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    out.world_normal = normalize(model.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_rgb = in.color.rgb;
    let object_a = in.color.a;

    let light_dir = normalize(light.dir);
    let light_color = light.color;

    let ambient_weight = 0.3;
    let diffuse_weight = 0.5;
    let specular_weight = 0.4;
    let shininess = 30.;

    let normal = normalize(in.world_normal);
    let diffuse_strength = max(dot(normal, light_dir), 0.0);

    let specular_strength = pow(diffuse_strength, shininess);

    let reflection = (ambient_weight + diffuse_weight * diffuse_strength + specular_weight * specular_strength) * light_color;
    let final_rgb = reflection * object_rgb;

    return vec4<f32>(final_rgb, object_a);
}
