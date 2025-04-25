/**
 * WebAssembly integration for delaunator-rs
 * Provides a similar API to the original JS Delaunator
 * @module delaunator-rs
 * @author Gabriel Maia
 * @license ISC
 */

// Importamos o módulo WebAssembly usando um caminho relativo
// Isso será substituído pelo bundle correto durante o build
let wasm_module;

/**
 * Initialize the WebAssembly module
 * @returns {Promise<Object>} The Delaunator module
 */
async function initDelaunator(wasm_path = './pkg/delaunator_rs.js') {
  try {
    // Importação dinâmica do módulo WASM
    const wasm = await import(wasm_path);
    // Inicializamos o módulo
    await wasm.default();
    wasm_module = wasm;
    console.log("Delaunator WebAssembly module initialized successfully");
    
    // Retornamos o objeto API compatível com a interface original
    return {
      /**
       * Construtor direto do Delaunator
       */
      Delaunator: wasm.Delaunator,
      
      /**
       * Create a Delaunator triangulation from an array of points
       * This provides API compatibility with the original JS Delaunator
       * @param {Array} points - Array of points [x,y] or objects
       * @param {Function} [getX] - Optional function to get X coordinate from custom point format
       * @param {Function} [getY] - Optional function to get Y coordinate from custom point format
       * @returns {Delaunator} The triangulation object
       */
      from: function(points, getX, getY) {
        // Validate input
        if (!points || !Array.isArray(points) || points.length === 0) {
          throw new Error("Points array is required and must not be empty");
        }
        
        // Handle custom accessor functions
        if (getX || getY) {
          try {
            const getXFn = getX || (p => p[0]);
            const getYFn = getY || (p => p[1]);
            const coords = new Float64Array(points.length * 2);
            
            for (let i = 0; i < points.length; i++) {
              const p = points[i];
              coords[2 * i] = getXFn(p);
              coords[2 * i + 1] = getYFn(p);
            }
            
            return new wasm.Delaunator(coords);
          } catch (err) {
            console.error("Error creating triangulation with custom accessors:", err);
            throw new Error(`Failed to create triangulation: ${err.message || String(err)}`);
          }
        }
        
        try {
          // Default case - use the from method
          return wasm.Delaunator.from(points);
        } catch (err) {
          console.error("Error creating triangulation:", err);
          throw new Error(`Failed to create triangulation: ${err.message || String(err)}`);
        }
      }
    };
  } catch (error) {
    console.error("Failed to initialize Delaunator WebAssembly module:", error);
    throw error;
  }
}

export default initDelaunator;
