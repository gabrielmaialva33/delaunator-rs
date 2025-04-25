/**
 * WebAssembly integration for delaunator-rs
 * Provides a similar API to the original JS Delaunator
 * @module delaunator-rs
 * @author Gabriel Maia
 * @license ISC
 */

// WebAssembly module
let wasm_module;

// Error message definitions
const ERROR_MESSAGES = {
  NO_WASM: 'WebAssembly is not available in your browser',
  INIT_FAILED: 'Failed to initialize Delaunator module',
  INVALID_POINTS: 'Invalid or empty points array',
  TRIANGULATION_FAILED: 'Failed to calculate triangulation',
};

/**
 * Initialize the WebAssembly module
 * @param {string} wasm_path - Path to the WASM module
 * @returns {Promise<Object>} The Delaunator module
 */
async function initDelaunator(wasm_path = './delaunator_rs.js') {
  try {
    // Check if WebAssembly is available
    if (typeof WebAssembly !== 'object') {
      throw new Error(ERROR_MESSAGES.NO_WASM);
    }

    // Load the WASM module
    console.log(`Loading Delaunator WebAssembly module from: ${wasm_path}`);
    const wasm = await import(wasm_path);
    
    // Inicializar o módulo
    await wasm.default();
    wasm_module = wasm;
    console.log("Delaunator WebAssembly module initialized successfully");
    
    // Retornar API completa
    return createAPI(wasm);
  } catch (error) {
    console.error("Failed to initialize Delaunator WebAssembly module:", error);
    throw new Error(`${ERROR_MESSAGES.INIT_FAILED}: ${error.message}`);
  }
}

/**
 * Create the API object with additional helper methods
 * @param {Object} wasm - The initialized WebAssembly module
 * @returns {Object} Enhanced API object
 */
function createAPI(wasm) {
  return {
    // Construtor básico
    Delaunator: wasm.Delaunator,
    
    /**
     * Create a Delaunator triangulation from an array of points
     * @param {Array} points - Array of points [x,y] or objects
     * @param {Function} [getX] - Optional function to get X coordinate from custom point format
     * @param {Function} [getY] - Optional function to get Y coordinate from custom point format
     * @returns {Delaunator} The triangulation object
     */
    from: function(points, getX, getY) {
      // Validate input
      if (!points || !Array.isArray(points) || points.length === 0) {
        throw new Error(ERROR_MESSAGES.INVALID_POINTS);
      }
      
      try {
        // Use custom accessors if provided
        if (getX || getY) {
          const getXFn = getX || (p => p[0]);
          const getYFn = getY || (p => p[1]);
          const coords = new Float64Array(points.length * 2);
          
          for (let i = 0; i < points.length; i++) {
            const p = points[i];
            coords[2 * i] = getXFn(p);
            coords[2 * i + 1] = getYFn(p);
          }
          
          return new wasm.Delaunator(coords);
        }
        
        // Caso padrão - use o método from
        return wasm.Delaunator.from(points);
      } catch (err) {
        console.error("Error creating triangulation:", err);
        throw new Error(`${ERROR_MESSAGES.TRIANGULATION_FAILED}: ${err.message || String(err)}`);
      }
    },
    
    /**
     * Create a triangulation from a set of 2D points
     * @param {Float64Array|Array<number>} coords - Flat array of coordinates [x0, y0, x1, y1, ...]
     * @returns {Delaunator} The triangulation object
     */
    fromCoords: function(coords) {
      if (!coords || coords.length < 6 || coords.length % 2 !== 0) {
        throw new Error(ERROR_MESSAGES.INVALID_POINTS);
      }
      
      try {
        // Ensure we have a Float64Array
        const coordsArray = coords instanceof Float64Array ? 
          coords : new Float64Array(coords);
        
        return new wasm.Delaunator(coordsArray);
      } catch (err) {
        console.error("Error creating triangulation from coords:", err);
        throw new Error(`${ERROR_MESSAGES.TRIANGULATION_FAILED}: ${err.message || String(err)}`);
      }
    },
    
    /**
     * Create a Voronoi diagram from a Delaunay triangulation
     * @param {Delaunator} delaunay - The Delaunay triangulation
     * @param {Array<number>} [bounds] - Optional bounds [minX, minY, maxX, maxY]
     * @returns {Object} Voronoi diagram object
     */
    voronoi: function(delaunay, bounds) {
      if (!delaunay || !delaunay.triangles) {
        throw new Error("Invalid Delaunay triangulation");
      }
      
      const triangles = delaunay.get_triangles();
      const coords = delaunay.get_coords();
      const hull = delaunay.get_hull();
      const n = coords.length >> 1;
      
      // Default bounds if not provided
      if (!bounds) {
        let minX = Infinity, minY = Infinity;
        let maxX = -Infinity, maxY = -Infinity;
        
        for (let i = 0; i < coords.length; i += 2) {
          const x = coords[i];
          const y = coords[i + 1];
          if (x < minX) minX = x;
          if (y < minY) minY = y;
          if (x > maxX) maxX = x;
          if (y > maxY) maxY = y;
        }
        
        // Add some padding
        const padding = Math.max(maxX - minX, maxY - minY) * 0.1;
        bounds = [
          minX - padding,
          minY - padding,
          maxX + padding,
          maxY + padding
        ];
      }
      
      // Implement a simple Voronoi diagram (just cell centers for now)
      const cells = new Array(n);
      for (let i = 0; i < n; i++) {
        cells[i] = [];
      }
      
      // Create Voronoi cells based on Delaunay triangulation
      // (Simple implementation - for a full version, a proper lib like d3-voronoi should be used)
      for (let i = 0; i < triangles.length; i += 3) {
        const a = triangles[i];
        const b = triangles[i + 1];
        const c = triangles[i + 2];
        
        // Calculate triangle circumcenter (this is a Voronoi vertex)
        const center = circumcenter(
          coords[2 * a], coords[2 * a + 1],
          coords[2 * b], coords[2 * b + 1],
          coords[2 * c], coords[2 * c + 1]
        );
        
        // Add the circumcenter to all cells of this triangle
        cells[a].push(center);
        cells[b].push(center);
        cells[c].push(center);
      }
      
      return {
        cells,
        bounds,
        coords,
        hull
      };
    },
    
    /**
     * Get the version information
     * @returns {Object} Version information
     */
    version: function() {
      return {
        name: 'delaunator-rs',
        version: '0.2.0',
        wasmAvailable: typeof WebAssembly === 'object',
        implementation: 'Rust + WebAssembly'
      };
    },
    
    /**
     * Utility function to bench performance
     * @param {number} numPoints - Number of points to generate
     * @param {number} [iterations=10] - Number of iterations for the benchmark
     * @returns {Promise<Object>} Benchmark results
     */
    bench: async function(numPoints, iterations = 10) {
      // Generate random points
      const points = [];
      for (let i = 0; i < numPoints; i++) {
        points.push([Math.random() * 1000, Math.random() * 1000]);
      }
      
      // Convert to flat array
      const coords = new Float64Array(points.length * 2);
      for (let i = 0; i < points.length; i++) {
        coords[i * 2] = points[i][0];
        coords[i * 2 + 1] = points[i][1];
      }
      
      // Run the benchmark
      const times = [];
      
      for (let i = 0; i < iterations; i++) {
        const start = performance.now();
        const delaunay = new wasm.Delaunator(coords);
        const end = performance.now();
        times.push(end - start);
      }
      
      // Calculate statistics
      times.sort((a, b) => a - b);
      const min = times[0];
      const max = times[times.length - 1];
      const avg = times.reduce((sum, t) => sum + t, 0) / times.length;
      const median = times[Math.floor(times.length / 2)];
      
      return {
        points: numPoints,
        iterations,
        min,
        max,
        avg,
        median,
        times
      };
    }
  };
}

/**
 * Calculate the circumcenter of a triangle
 * @param {number} ax - X coordinate of first point
 * @param {number} ay - Y coordinate of first point
 * @param {number} bx - X coordinate of second point
 * @param {number} by - Y coordinate of second point
 * @param {number} cx - X coordinate of third point
 * @param {number} cy - Y coordinate of third point
 * @returns {[number, number]} Coordinates of the circumcenter
 */
function circumcenter(ax, ay, bx, by, cx, cy) {
  const dx = bx - ax;
  const dy = by - ay;
  const ex = cx - ax;
  const ey = cy - ay;

  const bl = dx * dx + dy * dy;
  const cl = ex * ex + ey * ey;
  const d = 0.5 / (dx * ey - dy * ex);

  const x = ax + (ey * bl - dy * cl) * d;
  const y = ay + (dx * cl - ex * bl) * d;

  return [x, y];
}

export default initDelaunator;