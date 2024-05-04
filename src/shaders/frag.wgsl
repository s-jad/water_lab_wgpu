const PI: f32 = 3.14159265358979323846;
const MAX_F32: f32 = 0x1.fffffep+127f;

const SCREEN_WIDTH: f32 = 1376.0;
const SCREEN_HEIGHT: f32 = 768.0;
const ASPECT: f32 = SCREEN_WIDTH / SCREEN_HEIGHT;
const INV_ASPECT: f32 = SCREEN_HEIGHT / SCREEN_WIDTH;

const PLANT_REFLECTIVITY: f32 = 1.0;
const SAND_REFLECTIVITY: f32 = 1.0;
const ROCK_REFLECTIVITY: f32 = 1.0;
const WATER_REFLECTIVITY: f32 = 1.0;

const PLANT_CLR: vec3<f32> = vec3(1.0);
const SAND_CLR: vec3<f32> = vec3(1.0);
const ROCK_CLR: vec3<f32> = vec3(1.0);
const WATER_CLR: vec3<f32> = vec3(1.0);

const MAX_STEPS: i32 = 2500;

const m2: mat2x2<f32> = mat2x2(
  0.80, 0.60,
  -0.60, 0.80,
);

const m2Inv: mat2x2<f32> = mat2x2(
  0.80, -0.60,
  0.60, 0.80,
);

struct TimeUniform {
    time: f32,
};
struct RayParams {
  epsilon: f32,
  max_dist: f32,
  max_steps: f32,
}
struct ViewParams {
  x_shift: f32,
  y_shift: f32,
  zoom: f32,
  x_rot: f32,
  y_rot: f32,
  time_modifier: f32,
  fov: f32,
}

// GROUPS AND BINDINGS
@group(0) @binding(0) var<uniform> tu: TimeUniform;

@group(1) @binding(0) var<storage, read_write> rp: RayParams;
@group(1) @binding(1) var<storage, read_write> vp: ViewParams;
@group(1) @binding(7) var<storage, read_write> debug_arr1: array<vec4<f32>>;
@group(1) @binding(8) var<storage, read_write> debug_arr2: array<vec4<f32>>;
@group(1) @binding(9) var<storage, read_write> debug: vec4<f32>;

@group(2) @binding(0) var terrain_tex: texture_2d<f32>;
@group(2) @binding(1) var terrain_sampler: sampler;

// ASPECT RATIO
fn scale_aspect(fc: vec2<f32>) -> vec2<f32> {
  // Scale from screen dimensions to 0.0 --> 1.0
  var uv: vec2<f32> = ((2.0 * fc) / vec2(SCREEN_WIDTH, SCREEN_HEIGHT)) - 1.0;
  uv.y = -uv.y * INV_ASPECT;
  return uv;
}

// LIGHTING
fn get_normal(pos: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
  let e = vec2(rp.epsilon, 0.0);
  let n = vec3(map(pos, uv).dist) - 
  vec3(
    map(pos - e.xyy, uv).dist,
    map(pos - e.yxy, uv).dist,
    map(pos - e.yyx, uv).dist
  );

  return normalize(n);
}

fn get_ambient_occlusion(pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>) -> f32 {
  var occ = 0.0;
  var weight = 0.4;

  for (var i: i32 = 0; i < 8; i++) {
    let len = 0.01 + 0.02 * f32(i * i);
    let dist = map(pos + normal * len, uv).dist;
    occ += (len - dist) * weight;
    weight *= 0.85;
  }

  return 1.0 - clamp(0.6 * occ, 0.0, 1.0);
}

fn get_soft_shadow(pos: vec3<f32>, light_pos: vec3<f32>, uv: vec2<f32>) -> f32 {
  var res = 1.0;
  var dist = 0.01;
  let light_size = 100.0;

  for (var i: i32 = 0; i < 8; i++) {
    let hit = map(pos + light_pos * dist, uv).dist;
    res = min(res, hit / (dist * light_size));
    if (hit < rp.epsilon) { break; }
    dist += hit;
    if (dist > 50.0) { break; }
  }

  return clamp(res, 0.0, 1.0);
}

struct MaterialEnum {
  water: f32,
  rock: f32,
  plant: f32,
  sand: f32,
}

fn get_light(
  pos: vec3<f32>,
  rd: vec3<f32>,
  uv: vec2<f32>,
  material: MaterialEnum,
) -> vec3<f32> {
  var light_pos: vec3<f32> = vec3(40.0, 50.0, -500.0);
  let color: vec3<f32> = vec3(1.0);

  let l: vec3<f32> = normalize(light_pos - pos);
  let normal: vec3<f32> = get_normal(pos, uv);

  let v: vec3<f32> = -rd;
  let r: vec3<f32> = reflect(-l, normal);

  let diff: f32 = 0.70 * max(dot(l, normal), 0.0);
  let specular: f32 = 0.30 * pow(clamp(dot(r, v), 0.0, 1.0), 10.0);
  let ambient: f32 = 0.05; 

  var reflect: f32 = 0.0;
  reflect += material.water*WATER_REFLECTIVITY;
  reflect += material.rock*ROCK_REFLECTIVITY;
  reflect += material.plant*PLANT_REFLECTIVITY;
  reflect += material.sand*SAND_REFLECTIVITY;

  let spec_ref = specular*reflect;
  let diff_ref = diff*reflect;

  let shadow: f32 = get_soft_shadow(pos, light_pos, uv);
  let occ: f32 = get_ambient_occlusion(pos, normal, uv);

  return (ambient * occ + (spec_ref * occ + diff_ref) * shadow) * color;
}

// CAMERA

fn get_cam(ro: vec3<f32>, look_at: vec3<f32>) -> mat4x4<f32> {
  let camf = normalize(vec3(look_at - ro));
  let camr = normalize(cross(vec3(0.0, 1.0, 0.0), camf));
  let camu = cross(camf, camr);
  let camp = vec4(-ro.x, -ro.y, -ro.z, 1.0);

  return mat4x4(
    vec4(camr, 0.0), 
    vec4(camu, 0.0), 
    vec4(camf, 0.0), 
    camp
  );
}

fn rotate3d(v: vec3<f32>, angleX: f32, angleY: f32) -> vec3<f32> {
 let saX = sin(angleX);
 let caX = cos(angleX);
 let saY = sin(angleY);
 let caY = cos(angleY);

 // Rotation matrix for X-axis rotation
 let mtxX = mat3x3<f32>(
    1.0, 0.0, 0.0,
    0.0, caX, -saX,
    0.0, saX, caX
 );

 // Rotation matrix for Y-axis rotation
 let mtxY = mat3x3<f32>(
    caY, 0.0, saY,
    0.0, 1.0, 0.0,
    -saY, 0.0, caY
 );

 let rotatedX = v * mtxX;
 let rotatedY = rotatedX * mtxY;

 return rotatedY;
}

fn calculate_slope(pos: vec3<f32>, uv: vec2<f32>) -> f32 {
    let r_vec = normalize(pos);
    let n_vec = get_normal(pos, uv);
    let dp = dot(r_vec, n_vec);
    let angle = acos(dp);
    let slope_in_degrees = degrees(angle);

    return slope_in_degrees;
}

struct Terrain {
  grad: vec2<f32>,
  dist: f32,
  water_depth: f32,
}

fn map(pos: vec3<f32>, uv: vec2<f32>) -> Terrain {
  let tx = textureSample(terrain_tex, terrain_sampler, uv);
  var d1 = tx.x;
  var d0 = d1;
   
  // Calc water depth for use in render
  let water_depth = max(0.0, d1 - d0);
  // Cover lower elevations in water
  d1 = min(d0, d1);
  
  return Terrain(vec2(tx.y, tx.z), d1, water_depth);
}

// RAY MARCHING
struct TerrainPos {
  grad: vec2<f32>,
  dist: f32,
  water_depth: f32,
  pos: vec3<f32>,
}

fn ray_march(ro: vec3<f32>, rd: vec3<f32>, uv: vec2<f32>, look_at: vec3<f32>) -> TerrainPos {
  var dist = 0.0;
  var water_depth = 0.0;
  var grad = vec2(0.0);
  var p = vec3(0.0);

  for (var i: i32 = 0; i < MAX_STEPS; i++) {
    let pos = ro + dist * rd;
    let t = map(pos, uv);
    let hit = t.dist;
    water_depth = t.water_depth;
    grad = t.grad;
    p = pos;

    if (abs(hit) < rp.epsilon) {
      break;
    }
    dist += hit;

    if (dist > rp.max_dist) {
      break;
    }
  }

  return TerrainPos(grad, dist, water_depth, p);
}

// RENDERING
fn render(uv: vec2<f32>) -> vec3<f32> {
  var ro: vec3<f32> = vec3(0.0, 0.0, -300.0);
  ro = rotate3d(ro, vp.y_rot, vp.x_rot);

  let look_at: vec3<f32> = vec3(0.0, 0.0, 0.0);

  var rd: vec3<f32> = (get_cam(ro, look_at) * normalize(vec4(uv * vp.fov, 1.0, 0.0))).xyz;
  let terrain = ray_march(ro, rd, uv, look_at);
  let dist: f32 = terrain.dist;
  let grad = terrain.grad;

  let cam_pos = ro + dist * rd;
  var col: vec3<f32> = vec3(0.0);
  var material = MaterialEnum(0.0, 0.0, 0.0, 0.0);

  if (dist < rp.max_dist) {
    let dist_origin: f32 = length(cam_pos);
    material.rock = 1.0;
    col += get_light(cam_pos, rd, uv, material)*ROCK_CLR;
  }
  
  return col;
}

@fragment
fn main(@builtin(position) FragCoord: vec4<f32>) -> @location(0) vec4<f32> {
  let t: f32 = tu.time * vp.time_modifier;
  var uv: vec2<f32> = scale_aspect(FragCoord.xy); // Scale to -1.0 -> 1.0 + fix aspect ratio
  let uv0 = uv;
  uv.x += vp.x_shift * vp.zoom;
  uv.y += vp.y_shift * vp.zoom;
  uv /= vp.zoom;

  var color = vec3(0.0);
// -----------------------------------------------------------------------------------------------

  color = render(uv);

// -----------------------------------------------------------------------------------------------
  return vec4<f32>(color, 1.0);
}
