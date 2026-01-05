//! # Cache-Aware Loop Tiling
//!
//! Transforms loop nests to improve cache locality.
//! Breaks large iterations into smaller tiles that fit in L1/L2/L3 caches.

/// Cache level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLevel {
    L1,  // 32 KB
    L2,  // 256 KB
    L3,  // 8 MB
}

impl CacheLevel {
    /// Get cache size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            CacheLevel::L1 => 32 * 1024,
            CacheLevel::L2 => 256 * 1024,
            CacheLevel::L3 => 8 * 1024 * 1024,
        }
    }

    /// Get optimal tile size for this cache level
    pub fn optimal_tile_size(&self, element_size: usize) -> usize {
        // Leave 20% headroom for other data
        let usable = (self.size_bytes() * 80) / 100;
        usable / element_size
    }
}

/// Loop tiling configuration
#[derive(Debug, Clone)]
pub struct TilingConfig {
    /// Target cache level
    pub cache_level: CacheLevel,
    /// Array element size (bytes)
    pub element_size: usize,
    /// Original loop bounds
    pub original_bounds: (usize, usize),
    /// Tile size for outer loop
    pub tile_size: usize,
    /// Number of arrays accessed
    pub array_count: usize,
}

impl TilingConfig {
    /// Create new tiling configuration
    pub fn new(
        cache_level: CacheLevel,
        element_size: usize,
        start: usize,
        end: usize,
        array_count: usize,
    ) -> Self {
        let tile_size = cache_level.optimal_tile_size(element_size * array_count);

        TilingConfig {
            cache_level,
            element_size,
            original_bounds: (start, end),
            tile_size,
            array_count,
        }
    }

    /// Estimate cache misses before tiling
    pub fn original_cache_misses(&self) -> f32 {
        let (start, end) = self.original_bounds;
        let total_accesses = (end - start) as f32;
        
        // Rough estimate: 5-10% miss rate for large sequential access
        total_accesses * 0.08
    }

    /// Estimate cache misses after tiling
    pub fn tiled_cache_misses(&self) -> f32 {
        let (start, end) = self.original_bounds;
        let total_iterations = end - start;
        let num_tiles = (total_iterations + self.tile_size - 1) / self.tile_size;
        
        // Within-tile miss rate much lower (fits in cache)
        // Between-tile misses once per tile
        (num_tiles as f32) * 2.0 // Rough estimate
    }

    /// Estimate speedup from tiling
    pub fn speedup(&self) -> f32 {
        let original = self.original_cache_misses();
        let tiled = self.tiled_cache_misses();

        if original < 1.0 {
            return 1.0;
        }

        original / tiled.max(1.0)
    }
}

/// Loop tiling code generator
pub struct LoopTiler {
    pub config: TilingConfig,
    pub instructions: Vec<String>,
    label_counter: usize,
}

impl LoopTiler {
    /// Create new loop tiler
    pub fn new(config: TilingConfig) -> Self {
        LoopTiler {
            config,
            instructions: Vec::new(),
            label_counter: 0,
        }
    }

    /// Generate unique label
    fn gen_label(&mut self, prefix: &str) -> String {
        let label = format!("{}{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Generate tiled loop
    pub fn generate_tiled_loop(&mut self) -> String {
        let (start, end) = self.config.original_bounds;
        let tile_size = self.config.tile_size;

        let outer_label = self.gen_label(".tile_outer_");
        let outer_end = self.gen_label(".tile_outer_end_");
        let inner_label = self.gen_label(".tile_inner_");
        let inner_end = self.gen_label(".tile_inner_end_");

        self.instructions.push("    ; === Cache-Aware Loop Tiling ===".to_string());
        self.instructions.push(format!("    mov rax, {}          ; outer loop (tiles)", start));
        self.instructions.push(format!("    mov rbx, {}          ; tile size", tile_size));

        self.instructions.push(format!("{}:", outer_label));
        self.instructions.push(format!("    cmp rax, {}          ; check if done", end));
        self.instructions.push(format!("    jge {}", outer_end));

        // Calculate tile end
        self.instructions.push(format!("    mov rcx, rax"));
        self.instructions.push(format!("    add rcx, rbx         ; rcx = tile end"));
        self.instructions.push(format!("    cmp rcx, {}", end));
        self.instructions.push(format!("    cmovg rcx, {}        ; min(tile_end, loop_end)", end));

        // Inner loop (within tile)
        self.instructions.push(format!("{}:", inner_label));
        self.instructions.push(format!("    cmp rax, rcx         ; within tile?"));
        self.instructions.push(format!("    jge {}", inner_end));

        // Process element
        self.instructions.push("    mov rdx, [rax]       ; load element".to_string());
        self.instructions.push("    add rdx, 1           ; process".to_string());
        self.instructions.push("    mov [rax], rdx       ; store element".to_string());

        // Next iteration
        self.instructions.push("    add rax, 8           ; next element (8 bytes)".to_string());
        self.instructions.push(format!("    jmp {}", inner_label));

        self.instructions.push(format!("{}:", inner_end));

        // Next tile
        self.instructions.push(format!("    jmp {}", outer_label));

        self.instructions.push(format!("{}:", outer_end));
        self.instructions.push("    ; === Tiled Loop Complete ===".to_string());

        self.instructions.join("\n")
    }

    /// Generate 2D tiled loop (matrix access pattern)
    pub fn generate_2d_tiled_loop(
        &mut self,
        rows: usize,
        cols: usize,
    ) -> String {
        let tile_size = self.config.tile_size;
        let tile_rows = (tile_size as f32).sqrt() as usize;
        let tile_cols = tile_size / tile_rows;

        let outer_row_label = self.gen_label(".tile_row_outer_");
        let outer_row_end = self.gen_label(".tile_row_outer_end_");
        let outer_col_label = self.gen_label(".tile_col_outer_");
        let outer_col_end = self.gen_label(".tile_col_outer_end_");
        let inner_row_label = self.gen_label(".tile_row_inner_");
        let inner_row_end = self.gen_label(".tile_row_inner_end_");
        let inner_col_label = self.gen_label(".tile_col_inner_");
        let inner_col_end = self.gen_label(".tile_col_inner_end_");

        self.instructions.push("    ; === 2D Cache-Aware Loop Tiling ===".to_string());
        self.instructions.push(format!("    xor rax, rax         ; row tile counter"));
        self.instructions.push(format!("    mov r8, {}           ; tile row size", tile_rows));

        self.instructions.push(format!("{}:", outer_row_label));
        self.instructions.push(format!("    cmp rax, {}          ; done with rows?", rows));
        self.instructions.push(format!("    jge {}", outer_row_end));

        self.instructions.push(format!("    xor rbx, rbx         ; col tile counter"));
        self.instructions.push(format!("    mov r9, {}           ; tile col size", tile_cols));

        self.instructions.push(format!("{}:", outer_col_label));
        self.instructions.push(format!("    cmp rbx, {}          ; done with cols?", cols));
        self.instructions.push(format!("    jge {}", outer_col_end));

        // Inner row loop
        self.instructions.push(format!("    mov rcx, rax         ; row in tile"));
        self.instructions.push(format!("{}:", inner_row_label));
        self.instructions.push(format!("    cmp rcx, rax"));
        self.instructions.push(format!("    add rcx, r8          ; next row"));
        self.instructions.push(format!("    cmp rcx, {}", rows));
        self.instructions.push(format!("    jge {}", inner_row_end));

        // Inner col loop
        self.instructions.push(format!("    mov rdx, rbx         ; col in tile"));
        self.instructions.push(format!("{}:", inner_col_label));
        self.instructions.push(format!("    cmp rdx, rbx"));
        self.instructions.push(format!("    add rdx, r9          ; next col"));
        self.instructions.push(format!("    cmp rdx, {}", cols));
        self.instructions.push(format!("    jge {}", inner_col_end));

        // Process element
        self.instructions.push("    ; process matrix[rcx][rdx]".to_string());
        self.instructions.push("    mov rsi, rcx".to_string());
        self.instructions.push(format!("    imul rsi, {}         ; row offset", cols));
        self.instructions.push("    add rsi, rdx".to_string());
        self.instructions.push("    mov rdi, [rax + rsi * 8] ; load element".to_string());
        self.instructions.push("    add rdi, 1               ; process".to_string());
        self.instructions.push("    mov [rax + rsi * 8], rdi ; store element".to_string());

        self.instructions.push(format!("    jmp {}", inner_col_label));
        self.instructions.push(format!("{}:", inner_col_end));
        self.instructions.push(format!("    jmp {}", inner_row_label));
        self.instructions.push(format!("{}:", inner_row_end));

        // Next col tile
        self.instructions.push(format!("    add rbx, r9          ; next col tile"));
        self.instructions.push(format!("    jmp {}", outer_col_label));
        self.instructions.push(format!("{}:", outer_col_end));

        // Next row tile
        self.instructions.push(format!("    add rax, r8          ; next row tile"));
        self.instructions.push(format!("    jmp {}", outer_row_label));
        self.instructions.push(format!("{}:", outer_row_end));

        self.instructions.push("    ; === 2D Tiled Loop Complete ===".to_string());

        self.instructions.join("\n")
    }

    /// Get generated assembly
    pub fn get_assembly(&self) -> String {
        self.instructions.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_level_sizes() {
        assert_eq!(CacheLevel::L1.size_bytes(), 32 * 1024);
        assert_eq!(CacheLevel::L2.size_bytes(), 256 * 1024);
        assert_eq!(CacheLevel::L3.size_bytes(), 8 * 1024 * 1024);
    }

    #[test]
    fn test_optimal_tile_size() {
        let l1 = CacheLevel::L1;
        let tile_size = l1.optimal_tile_size(8); // 8-byte elements
        assert!(tile_size > 0);
        assert!(tile_size < l1.size_bytes());
    }

    #[test]
    fn test_tiling_config_creation() {
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 1000, 1);
        assert_eq!(config.original_bounds, (0, 1000));
        assert!(config.tile_size > 0);
    }

    #[test]
    fn test_speedup_estimation() {
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 10000, 4);
        let speedup = config.speedup();
        assert!(speedup >= 1.0);
    }

    #[test]
    fn test_loop_tiler_generation() {
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 1000, 1);
        let mut tiler = LoopTiler::new(config);
        let asm = tiler.generate_tiled_loop();
        
        assert!(asm.contains("tile"));
        assert!(asm.contains("outer"));
        assert!(asm.contains("inner"));
    }

    #[test]
    fn test_2d_tiling_generation() {
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 1000, 2);
        let mut tiler = LoopTiler::new(config);
        let asm = tiler.generate_2d_tiled_loop(100, 100);
        
        assert!(asm.contains("2D"));
        assert!(asm.contains("row"));
        assert!(asm.contains("col"));
    }
}
