// Exclusive prefix sum in several passes (reduce, reduce, serial top, down, down), blocks of 256.
// Needs no forward-progress guarantee, so it is portable. Array lengths: a = n0 (multiple of 256),
// b = n0 / 256 rounded up to a multiple of 256, c = len(b) / 256. Result in `o`.
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> b: array<u32>;
@group(0) @binding(2) var<storage, read_write> c: array<u32>;
@group(0) @binding(3) var<storage, read_write> o: array<u32>;

const B = 256u;

@compute @workgroup_size(64)
fn reduce1(@builtin(global_invocation_id) g: vec3<u32>) {
    let k = g.x;
    if (k >= arrayLength(&b)) { return; }
    var sum = 0u;
    if (k * B < arrayLength(&a)) {
        for (var t = 0u; t < B; t++) { sum += a[k * B + t]; }
    }
    b[k] = sum;
}

@compute @workgroup_size(64)
fn reduce2(@builtin(global_invocation_id) g: vec3<u32>) {
    let k = g.x;
    if (k >= arrayLength(&c)) { return; }
    var sum = 0u;
    for (var t = 0u; t < B; t++) { sum += b[k * B + t]; }
    c[k] = sum;
}

@compute @workgroup_size(1)
fn scan_top() {
    var run = 0u;
    let n = arrayLength(&c);
    for (var k = 0u; k < n; k++) {
        let v = c[k];
        c[k] = run;
        run += v;
    }
}

@compute @workgroup_size(64)
fn down1(@builtin(global_invocation_id) g: vec3<u32>) {
    let k = g.x;
    if (k >= arrayLength(&c)) { return; }
    var run = c[k];
    for (var t = 0u; t < B; t++) {
        let v = b[k * B + t];
        b[k * B + t] = run;
        run += v;
    }
}

@compute @workgroup_size(64)
fn down0(@builtin(global_invocation_id) g: vec3<u32>) {
    let k = g.x;
    if (k * B >= arrayLength(&a)) { return; }
    var run = b[k];
    for (var t = 0u; t < B; t++) {
        o[k * B + t] = run;
        run += a[k * B + t];
    }
}
