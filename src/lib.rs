//! Delaunator Rust implementation for Delaunay triangulation
//!
//! This crate provides a fast, robust Delaunay triangulation algorithm for 2D points.
//! It is designed to work with both native Rust and WebAssembly.

mod utils;

// Required imports for WebAssembly bindings
use wasm_bindgen::prelude::*;

// Conditional logging for debugging
#[cfg(feature = "debug")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[cfg(feature = "debug")]
#[allow(unused_macros)]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)));
}

#[cfg(not(feature = "debug"))]
#[allow(unused_macros)]
macro_rules! console_log {
    ($($t:tt)*) => {};
}

// Constant equivalent to JavaScript's EPSILON (2^-52)
const EPSILON: f64 = 2.220446049250313e-16;
// Fixed size stack for edge legalization
const EDGE_STACK_SIZE: usize = 512;

/// Delaunator struct for Delaunay triangulation
///
/// This struct holds both the input coordinates and the output triangulation data.
/// It provides methods for creating and updating triangulations.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Delaunator {
    // Input coordinates [x0, y0, x1, y1, ...]
    pub(crate) coords: Vec<f64>,

    // Public outputs
    #[wasm_bindgen(skip)]
    pub triangles: Vec<u32>,
    #[wasm_bindgen(skip)]
    pub halfedges: Vec<i32>,
    #[wasm_bindgen(skip)]
    pub hull: Vec<u32>,

    // Private internal state
    triangles_len: usize,
    hull_start: usize,
    hash_size: usize,
    hull_prev: Vec<u32>,
    hull_next: Vec<u32>,
    hull_tri: Vec<u32>,
    hull_hash: Vec<i32>,
    ids: Vec<u32>,
    dists: Vec<f64>,
    cx: f64,
    cy: f64,
}

#[wasm_bindgen]
impl Delaunator {
    /// Creates a new Delaunator instance from a flat array of point coordinates
    ///
    /// The input format should be a flat array of coordinates [x0, y0, x1, y1, ...]
    /// Returns error if the input is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(coords: Vec<f64>) -> Result<Delaunator, JsValue> {
        // Initialize WebAssembly utils
        utils::initialize();
        let n = coords.len() >> 1;

        if n == 0 || coords.len() % 2 != 0 {
            return Err(JsValue::from_str("Invalid coordinates array"));
        }

        if n > 0 && !coords[0].is_finite() {
            return Err(JsValue::from_str("Expected coords to contain numbers"));
        }

        // Maximum possible number of triangles
        let max_triangles = std::cmp::max(2 * n - 5, 0);

        // Initialize all arrays
        let hash_size = n.next_power_of_two() / 2; // Similar to Math.ceil(Math.sqrt(n))

        let mut delaunator = Delaunator {
            coords,
            triangles: vec![0; max_triangles * 3],
            halfedges: vec![-1; max_triangles * 3],
            hull: Vec::new(),

            triangles_len: 0,
            hull_start: 0,
            hash_size,
            hull_prev: vec![0; n],
            hull_next: vec![0; n],
            hull_tri: vec![0; n],
            hull_hash: vec![-1; hash_size],
            ids: vec![0; n],
            dists: vec![0.0; n],
            cx: 0.0,
            cy: 0.0,
        };

        delaunator.update();
        Ok(delaunator)
    }

    /// Creates a Delaunator instance from an array of points
    ///
    /// This is a static factory method for JavaScript/WebAssembly users.
    /// It accepts an array of points and converts them to the flat
    /// coordinate format required by the algorithm.
    ///
    /// Example: `Delaunator.from([[0,0], [1,0], [0,1]]);`
    #[wasm_bindgen(js_name = "from")]
    pub fn from(points: &JsValue) -> Result<Delaunator, JsValue> {
        // Initialize WebAssembly utils
        utils::initialize();
        // Verifica se é um array e converte
        if !js_sys::Array::is_array(points) {
            return Err(JsValue::from_str("Expected points to be an array"));
        }

        // Converte o JsValue para um Array javascript
        let points_array = js_sys::Array::from(points);

        // Verificação de segurança adicional
        if points_array.length() == 0 {
            // Array vazio é válido, retorna uma triangulação vazia
            return Delaunator::new(Vec::new());
        }
        let n = points_array.length() as usize;

        if n == 0 {
            return Delaunator::new(Vec::new());
        }

        let mut coords = Vec::with_capacity(n * 2);

        for i in 0..n {
            // Get point from array (safely)
            let i_val = JsValue::from(i);
            let point = match js_sys::Reflect::get(&points_array, &i_val) {
                Ok(p) => p,
                Err(_) => {
                    return Err(JsValue::from_str(&format!(
                        "Failed to get point at index {}",
                        i
                    )))
                }
            };

            // Simplificando a verificação do formato do ponto
            if !js_sys::Array::is_array(&point)
                && !js_sys::Reflect::has(&point, &JsValue::from("length")).unwrap_or(false)
            {
                return Err(JsValue::from_str(&format!(
                    "Point at index {} is not an array-like object",
                    i
                )));
            }

            // Default behavior: get x and y from point[0] and point[1]
            // Não precisamos destas variáveis já que estamos acessando diretamente

            // Simplificando o acesso às coordenadas
            let x = js_sys::Reflect::get(&point, &JsValue::from(0))
                .map_err(|_| {
                    JsValue::from_str(&format!(
                        "Failed to get x-coordinate from point at index {}",
                        i
                    ))
                })?
                .as_f64()
                .ok_or_else(|| {
                    JsValue::from_str(&format!(
                        "Invalid x-coordinate at index {}: must be a number",
                        i
                    ))
                })?;

            let y = js_sys::Reflect::get(&point, &JsValue::from(1))
                .map_err(|_| {
                    JsValue::from_str(&format!(
                        "Failed to get y-coordinate from point at index {}",
                        i
                    ))
                })?
                .as_f64()
                .ok_or_else(|| {
                    JsValue::from_str(&format!(
                        "Invalid y-coordinate at index {}: must be a number",
                        i
                    ))
                })?;

            coords.push(x);
            coords.push(y);
        }

        Delaunator::new(coords)
    }

    /// Updates the triangulation when points have been modified in-place
    #[wasm_bindgen(js_name = "update")]
    pub fn update(&mut self) {
        let n = self.coords.len() >> 1;

        // Bail if there are fewer than 3 points (not enough for triangulation)
        if n < 3 {
            self.triangles = Vec::new();
            self.halfedges = Vec::new();
            self.hull = (0..n as u32).collect();
            return;
        }

        // Initialize point indices for sorting
        for i in 0..n {
            self.ids[i] = i as u32;
        }

        // Find the bounding box and a point close to the centroid
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for i in 0..n {
            let x = self.coords[2 * i];
            let y = self.coords[2 * i + 1];
            if x < min_x {
                min_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if x > max_x {
                max_x = x;
            }
            if y > max_y {
                max_y = y;
            }
        }

        // Calculate centroid
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;

        // Find the point closest to the centroid
        let mut min_dist = f64::INFINITY;
        let mut i0 = 0;

        for i in 0..n {
            let d = dist(cx, cy, self.coords[2 * i], self.coords[2 * i + 1]);
            if d < min_dist {
                i0 = i;
                min_dist = d;
            }
        }

        let i0x = self.coords[2 * i0];
        let i0y = self.coords[2 * i0 + 1];

        // Find the point closest to the first point
        min_dist = f64::INFINITY;
        let mut i1 = 0;

        for i in 0..n {
            if i == i0 {
                continue;
            }
            let d = dist(i0x, i0y, self.coords[2 * i], self.coords[2 * i + 1]);
            if d < min_dist && d > 0.0 {
                i1 = i;
                min_dist = d;
            }
        }

        let mut i1x = self.coords[2 * i1];
        let mut i1y = self.coords[2 * i1 + 1];

        // Find the third point which forms the smallest circumcircle
        let mut min_radius = f64::INFINITY;
        let mut i2 = 0;

        for i in 0..n {
            if i == i0 || i == i1 {
                continue;
            }
            let r = circumradius(
                i0x,
                i0y,
                i1x,
                i1y,
                self.coords[2 * i],
                self.coords[2 * i + 1],
            );
            if r < min_radius {
                i2 = i;
                min_radius = r;
            }
        }

        let mut i2x = self.coords[2 * i2];
        let mut i2y = self.coords[2 * i2 + 1];

        // Handle collinear case (all points on a line)
        if min_radius == f64::INFINITY {
            // Order points by dx (or dy if all x are identical)
            for i in 0..n {
                self.dists[i] = self.coords[2 * i] - self.coords[0];
                if self.dists[i] == 0.0 {
                    self.dists[i] = self.coords[2 * i + 1] - self.coords[1];
                }
            }

            quicksort(&mut self.ids, &mut self.dists, 0, n - 1);

            let mut hull = Vec::with_capacity(n);
            let _j = 0; // Variável não utilizada
            let mut d0 = f64::NEG_INFINITY;

            for i in 0..n {
                let id = self.ids[i] as usize;
                let d = self.dists[id];
                if d > d0 {
                    hull.push(id as u32);
                    d0 = d;
                }
            }

            self.hull = hull;
            self.triangles = Vec::new();
            self.halfedges = Vec::new();
            return;
        }

        // Ensure counterclockwise orientation for the first three points
        let orientation = orient2d(i0x, i0y, i1x, i1y, i2x, i2y);
        if orientation < 0.0 {
            // Swap the order of the second and third points
            std::mem::swap(&mut i1, &mut i2);
            std::mem::swap(&mut i1x, &mut i2x);
            std::mem::swap(&mut i1y, &mut i2y);
        }

        // Calculate the circumcenter of the first triangle
        let center = circumcenter(i0x, i0y, i1x, i1y, i2x, i2y);
        self.cx = center.0;
        self.cy = center.1;

        // Sort the points by distance from the circumcenter
        for i in 0..n {
            self.dists[i] = dist(
                self.coords[2 * i],
                self.coords[2 * i + 1],
                center.0,
                center.1,
            );
        }

        quicksort(&mut self.ids, &mut self.dists, 0, n - 1);

        // Set up the initial triangle as the starting hull
        self.hull_start = i0;
        let mut hull_size = 3;

        self.hull_next[i0] = i1 as u32;
        self.hull_prev[i2] = i1 as u32;
        self.hull_next[i1] = i2 as u32;
        self.hull_prev[i0] = i2 as u32;
        self.hull_next[i2] = i0 as u32;
        self.hull_prev[i1] = i0 as u32;

        self.hull_tri[i0] = 0;
        self.hull_tri[i1] = 1;
        self.hull_tri[i2] = 2;

        // Fill hash with initial triangle edges
        for i in 0..self.hash_size {
            self.hull_hash[i] = -1;
        }

        // Calcular hash_key uma vez para evitar múltiplos empréstimos
        let key_i0 = self.hash_key(i0x, i0y);
        let key_i1 = self.hash_key(i1x, i1y);
        let key_i2 = self.hash_key(i2x, i2y);

        self.hull_hash[key_i0] = i0 as i32;
        self.hull_hash[key_i1] = i1 as i32;
        self.hull_hash[key_i2] = i2 as i32;

        // Reset triangulation state
        self.triangles_len = 0;

        // Create the first triangle
        self.add_triangle(i0 as u32, i1 as u32, i2 as u32, -1, -1, -1);

        // Process remaining points
        let mut xp = 0.0;
        let mut yp = 0.0;

        for k in 0..self.ids.len() {
            let i = self.ids[k] as usize;
            let x = self.coords[2 * i];
            let y = self.coords[2 * i + 1];

            // Skip near-duplicate points
            if k > 0 && f64::abs(x - xp) <= EPSILON && f64::abs(y - yp) <= EPSILON {
                continue;
            }
            xp = x;
            yp = y;

            // Skip seed triangle points
            if i == i0 || i == i1 || i == i2 {
                continue;
            }

            // Find a visible edge on the convex hull using edge hash
            let mut start = 0;
            let key = self.hash_key(x, y);

            for j in 0..self.hash_size {
                let idx = (key + j) % self.hash_size;
                start = self.hull_hash[idx];
                if start != -1 && start != self.hull_next[start as usize] as i32 {
                    break;
                }
            }

            start = self.hull_prev[start as usize] as i32;
            let mut e = start;
            let mut q;

            // Find the visible edges on the convex hull
            loop {
                q = self.hull_next[e as usize] as i32;
                if orient2d(
                    x,
                    y,
                    self.coords[2 * e as usize],
                    self.coords[2 * e as usize + 1],
                    self.coords[2 * q as usize],
                    self.coords[2 * q as usize + 1],
                ) >= 0.0
                {
                    e = q;
                    if e == start {
                        e = -1;
                        break;
                    }
                } else {
                    break;
                }
            }

            // Skip if no visible edges found
            if e == -1 {
                continue;
            }

            // Add the first triangle from the point
            let mut t = self.add_triangle(
                e as u32,
                i as u32,
                self.hull_next[e as usize],
                -1,
                -1,
                self.hull_tri[e as usize] as i32,
            );

            // Recursively flip triangles from the point until they satisfy the Delaunay condition
            self.hull_tri[i] = self.legalize(t + 2);
            self.hull_tri[e as usize] = t as u32;
            hull_size += 1;

            // Walk forward through the hull, adding more triangles and flipping recursively
            let mut n = self.hull_next[e as usize] as i32;
            loop {
                q = self.hull_next[n as usize] as i32;
                if orient2d(
                    x,
                    y,
                    self.coords[2 * n as usize],
                    self.coords[2 * n as usize + 1],
                    self.coords[2 * q as usize],
                    self.coords[2 * q as usize + 1],
                ) < 0.0
                {
                    t = self.add_triangle(
                        n as u32,
                        i as u32,
                        q as u32,
                        self.hull_tri[i] as i32,
                        -1,
                        self.hull_tri[n as usize] as i32,
                    );
                    self.hull_tri[i] = self.legalize(t + 2);
                    self.hull_next[n as usize] = n as u32; // mark as removed
                    hull_size -= 1;
                    n = q;
                } else {
                    break;
                }
            }

            // Walk backward through the hull, adding more triangles and flipping
            if e == start {
                loop {
                    q = self.hull_prev[e as usize] as i32;
                    if orient2d(
                        x,
                        y,
                        self.coords[2 * q as usize],
                        self.coords[2 * q as usize + 1],
                        self.coords[2 * e as usize],
                        self.coords[2 * e as usize + 1],
                    ) < 0.0
                    {
                        t = self.add_triangle(
                            q as u32,
                            i as u32,
                            e as u32,
                            -1,
                            self.hull_tri[e as usize] as i32,
                            self.hull_tri[q as usize] as i32,
                        );
                        self.legalize(t + 2);
                        self.hull_tri[q as usize] = t as u32;
                        self.hull_next[e as usize] = e as u32; // mark as removed
                        hull_size -= 1;
                        e = q;
                    } else {
                        break;
                    }
                }
            }

            // Update hull indices
            self.hull_start = e as usize;
            self.hull_prev[i] = e as u32;
            self.hull_next[e as usize] = i as u32;
            self.hull_prev[n as usize] = i as u32;
            self.hull_next[i] = n as u32;

            // Save the two new edges in the hash table
            // Calcular hash_key uma vez para evitar múltiplos empréstimos
            let key_xy = self.hash_key(x, y);
            let e_coords_x = self.coords[2 * e as usize];
            let e_coords_y = self.coords[2 * e as usize + 1];
            let key_e = self.hash_key(e_coords_x, e_coords_y);

            self.hull_hash[key_xy] = i as i32;
            self.hull_hash[key_e] = e;
        }

        // Extract the hull as an array of point indices
        self.hull = Vec::with_capacity(hull_size);
        let mut e = self.hull_start;
        for _ in 0..hull_size {
            self.hull.push(e as u32);
            e = self.hull_next[e] as usize;
        }

        // Trim arrays to the actual number of triangles
        self.triangles.truncate(self.triangles_len);
        self.halfedges.truncate(self.triangles_len);
    }

    // JavaScript API methods for web use

    /// Get triangulation result as array of indices
    ///
    /// Returns a Uint32Array containing indices that form triangles
    /// (each group of 3 values represents a triangle)
    #[wasm_bindgen(getter, js_name = "triangles")]
    pub fn get_triangles(&self) -> js_sys::Uint32Array {
        let array = js_sys::Uint32Array::new_with_length(self.triangles.len() as u32);
        array.copy_from(&self.triangles);
        array
    }

    /// Get halfedges result as array
    ///
    /// Returns an Int32Array containing halfedge indices
    /// that allow traversal of the triangulation
    #[wasm_bindgen(getter, js_name = "halfedges")]
    pub fn get_halfedges(&self) -> js_sys::Int32Array {
        let array = js_sys::Int32Array::new_with_length(self.halfedges.len() as u32);
        array.copy_from(&self.halfedges);
        array
    }

    /// Get convex hull result as array
    ///
    /// Returns a Uint32Array containing indices of points
    /// that form the convex hull of the input
    #[wasm_bindgen(getter, js_name = "hull")]
    pub fn get_hull(&self) -> js_sys::Uint32Array {
        let array = js_sys::Uint32Array::new_with_length(self.hull.len() as u32);
        array.copy_from(&self.hull);
        array
    }

    /// Get input coordinates as array
    ///
    /// Returns a Float64Array containing the input coordinates
    /// in the format [x0, y0, x1, y1, ...]
    #[wasm_bindgen(getter, js_name = "coords")]
    pub fn get_coords(&self) -> js_sys::Float64Array {
        let array = js_sys::Float64Array::new_with_length(self.coords.len() as u32);
        array.copy_from(&self.coords);
        array
    }
}

// Private methods for Delaunator
impl Delaunator {
    // Calculate a hash key for a point (used in finding visible edges on the hull)
    fn hash_key(&self, x: f64, y: f64) -> usize {
        let dx = x - self.cx;
        let dy = y - self.cy;
        let p = (pseudo_angle(dx, dy) * self.hash_size as f64).floor() as usize;
        p % self.hash_size
    }

    // Add a triangle to the triangulation
    fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32, a: i32, b: i32, c: i32) -> usize {
        let t = self.triangles_len;

        self.triangles[t] = i0;
        self.triangles[t + 1] = i1;
        self.triangles[t + 2] = i2;

        self.link(t, a);
        self.link(t + 1, b);
        self.link(t + 2, c);

        self.triangles_len += 3;

        t
    }

    // Link two halfedges
    fn link(&mut self, a: usize, b: i32) {
        self.halfedges[a] = b;
        if b != -1 {
            self.halfedges[b as usize] = a as i32;
        }
    }

    // Recursively legalize triangles to maintain the Delaunay property
    fn legalize(&mut self, a: usize) -> u32 {
        let mut edge_stack = [0u32; EDGE_STACK_SIZE];
        let mut stack_size = 0;
        let mut ar = a;

        // Recursion eliminated with a fixed-size stack
        loop {
            let b = self.halfedges[ar] as i32;

            // If the pair of triangles doesn't satisfy the Delaunay condition,
            // flip them, then do the same check/flip recursively for the new pair
            let a0 = (ar / 3) * 3;
            ar = a0 + (ar + 2) % 3;

            if b == -1 {
                // Convex hull edge
                if stack_size == 0 {
                    break;
                }
                stack_size -= 1;
                ar = edge_stack[stack_size] as usize;
                continue;
            }

            let b0 = (b as usize / 3) * 3;
            let al = a0 + (ar + 1) % 3;
            let bl = b0 + (b as usize + 2) % 3;

            let p0 = self.triangles[ar] as usize;
            let pr = self.triangles[a0 + (ar + 1) % 3] as usize;
            let pl = self.triangles[al] as usize;
            let p1 = self.triangles[bl] as usize;

            // Check if the Delaunay condition is violated
            let illegal = in_circle(
                self.coords[2 * p0],
                self.coords[2 * p0 + 1],
                self.coords[2 * pr],
                self.coords[2 * pr + 1],
                self.coords[2 * pl],
                self.coords[2 * pl + 1],
                self.coords[2 * p1],
                self.coords[2 * p1 + 1],
            );

            if illegal {
                // Flip the edge
                self.triangles[ar] = p1 as u32;
                self.triangles[b as usize] = p0 as u32;

                let hbl = self.halfedges[bl] as i32;

                // Edge swapped on the other side of the hull (rare)
                // Fix the halfedge reference
                if hbl == -1 {
                    let mut e = self.hull_start as i32;
                    loop {
                        if self.hull_tri[e as usize] as usize == bl {
                            self.hull_tri[e as usize] = ar as u32;
                            break;
                        }
                        e = self.hull_prev[e as usize] as i32;
                        if e == self.hull_start as i32 {
                            break;
                        }
                    }
                }

                self.link(ar, hbl);
                self.link(b as usize, self.halfedges[ar] as i32);
                self.link(ar, bl as i32);

                let br = b0 + (b as usize + 1) % 3;

                if stack_size < EDGE_STACK_SIZE {
                    edge_stack[stack_size] = br as u32;
                    stack_size += 1;
                }
            } else {
                if stack_size == 0 {
                    break;
                }
                stack_size -= 1;
                ar = edge_stack[stack_size] as usize;
            }
        }

        ar as u32
    }
}

// Helper geometric functions

/// Calculate a pseudo-angle for sorting points around a point
///
/// This function calculates a value that increases monotonically
/// with the actual angle, but doesn't use expensive trigonometry.
/// Result range is [0..1].
/// Calculate a pseudo-angle for sorting points around a point
///
/// This function calculates a value that increases monotonically
/// with the actual angle, but doesn't use expensive trigonometry.
/// Result range is [0..1].
#[inline]
fn pseudo_angle(dx: f64, dy: f64) -> f64 {
    // Avoid division by zero with a safer epsilon
    let denom = f64::abs(dx) + f64::abs(dy);
    if denom < f64::EPSILON * 10.0 {
        return 0.0;
    }

    let p = dx / denom;
    let result = if dy > 0.0 { 3.0 - p } else { 1.0 + p };
    result / 4.0 // Range [0..1]
}

/// Calculate squared distance between two points
///
/// Returns the squared Euclidean distance for better performance
/// (avoiding a square root operation).
#[inline]
fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Determine if a point p is inside the circumcircle of a, b, c
/// This is a key predicate for the Delaunay condition
#[inline]
fn in_circle(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, px: f64, py: f64) -> bool {
    let dx = ax - px;
    let dy = ay - py;
    let ex = bx - px;
    let ey = by - py;
    let fx = cx - px;
    let fy = cy - py;

    let ap = dx * dx + dy * dy;
    let bp = ex * ex + ey * ey;
    let cp = fx * fx + fy * fy;

    let det = dx * (ey * cp - bp * fy) - dy * (ex * cp - bp * fx) + ap * (ex * fy - ey * fx);

    det < 0.0
}

/// Calculate the orientation of three points (clockwise, counterclockwise, or collinear)
///
/// Returns a positive value if the points are in counterclockwise order,
/// negative if clockwise, and zero if collinear.
#[inline]
fn orient2d(px: f64, py: f64, qx: f64, qy: f64, rx: f64, ry: f64) -> f64 {
    (qy - py) * (rx - qx) - (qx - px) * (ry - qy)
}

/// Calculate radius of the circumcircle of a triangle
fn circumradius(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let ex = cx - ax;
    let ey = cy - ay;

    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);

    let x = (ey * bl - dy * cl) * d;
    let y = (dx * cl - ex * bl) * d;

    x * x + y * y
}

/// Calculate circumcenter of a triangle
///
/// Returns the x,y coordinates of the center of the circle passing through
/// the three triangle vertices.
fn circumcenter(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> (f64, f64) {
    let dx = bx - ax;
    let dy = by - ay;
    let ex = cx - ax;
    let ey = cy - ay;

    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);

    let x = ax + (ey * bl - dy * cl) * d;
    let y = ay + (dx * cl - ex * bl) * d;

    (x, y)
}

/// QuickSort implementation for sorting points by distance
///
/// This sorts the ids array based on values in the dists array.
/// Uses insertion sort for small arrays to improve performance.
fn quicksort(ids: &mut [u32], dists: &mut [f64], left: usize, right: usize) {
    if right <= left {
        return;
    }

    if right - left <= 20 {
        // Insertion sort for small arrays
        for i in left + 1..=right {
            let temp = ids[i];
            let temp_dist = dists[temp as usize];
            let mut j = i;
            while j > left && dists[ids[j - 1] as usize] > temp_dist {
                ids[j] = ids[j - 1];
                j -= 1;
            }
            ids[j] = temp;
        }
    } else {
        // QuickSort for larger arrays
        let median = (left + right) >> 1;
        let mut i = left + 1;
        let mut j = right;

        swap(ids, median, i);

        if dists[ids[left] as usize] > dists[ids[right] as usize] {
            swap(ids, left, right);
        }
        if dists[ids[i] as usize] > dists[ids[right] as usize] {
            swap(ids, i, right);
        }
        if dists[ids[left] as usize] > dists[ids[i] as usize] {
            swap(ids, left, i);
        }

        let temp = ids[i];
        let temp_dist = dists[temp as usize];

        let mut running = true;
        while running {
            loop {
                i += 1;
                if i >= right || dists[ids[i] as usize] >= temp_dist {
                    break;
                }
            }
            loop {
                j -= 1;
                if j <= left || dists[ids[j] as usize] <= temp_dist {
                    break;
                }
            }
            if j < i {
                running = false;
            } else {
                swap(ids, i, j);
            }
        }

        ids[left + 1] = ids[j];
        ids[j] = temp;

        if right - i + 1 >= j - left {
            quicksort(ids, dists, i, right);
            quicksort(ids, dists, left, j.saturating_sub(1));
        } else {
            quicksort(ids, dists, left, j.saturating_sub(1));
            quicksort(ids, dists, i, right);
        }
    }
}

// Helper function to swap elements in an array
fn swap(arr: &mut [u32], i: usize, j: usize) {
    let temp = arr[i];
    arr[i] = arr[j];
    arr[j] = temp;
}

// Removendo a struct JsError que pode estar causando conflitos
