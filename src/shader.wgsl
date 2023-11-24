
struct PrimeIndices {
  //data: [[stride(4)]] array<u32>;
  data: array<u32>,
}; // this is used as both input and output for convenience


struct NumSeq {
  data: array<vec4<f32>>,
}

@group(0) @binding(0)
//var<storage, read_write> v_indices: PrimeIndices;
var<storage, read_write> v_numseq: NumSeq;

// The Collatz Conjecture states that for any integer n:
// If n is even, n = n/2
// If n is odd, n = 3n+1
// And repeat this process for each new n, you will always eventually reach 1.
// Though the conjecture has not been proven, no counterexample has ever been found.
// This function returns how many times this recurrence needs to be applied to reach 1.
fn collatz_iterations(n_base: u32) -> u32{
  var n: u32 = n_base;
  var i: u32 = 0u;
  loop {
    if (n <= 1u) {
      break;
    }
    if (n % 2u == 0u) {
      n = n / 2u;
    }
    else {
      // Overflow? (i.e. 3*n + 1 > 0xffffffffu?)
      if (n >= 1431655765u) {   // 0x55555555u
        return 4294967295u;   // 0xffffffffu
      }

      n = 3u * n + 1u;
    }
    i = i + 1u;
  }
  return i;
}

fn float_function(a: vec4<f32>) -> vec4<f32> {
  //var b = vec4<f32>(a.x, a.y, a.z, a.w);
  var b = vec4<f32>(a.x * 1.5, a.y * 1.7, a.z, a.w);
  return b;
}

const LIMIT = 256;
const THRE  = 4.0;

fn mandel(c: vec4<f32>) -> vec4<f32> {
  var a: f32 = 0.0;
  var b: f32 = 0.0;
  var d: f32 = 0.0;

  for (var i = 0; i < LIMIT; i++) {
    let a2 = a * a - (b * b) + c.x;
    let b2 = 2.0 * a * b + c.y;
    a = a2;
    b = b2;
    d = a * a + b * b;
    if (d > THRE) {
      let g = f32(i) * 5.0;
      //let h = 255.0 / (1.0 + exp(-log(d)));
      //return ret;
      return vec4(g, g, 0.0, 250.0);
    }
  }
  //return ret;
  return vec4(0.0, 0.0, 0.0, 250.0);
}

@compute @workgroup_size(64)
/*
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  v_indices.data[global_id.x] = collatz_iterations(v_indices.data[global_id.x]);
}
*/
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  v_numseq.data[global_id.x] = mandel(v_numseq.data[global_id.x]);
}
