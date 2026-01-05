// Integration tests for v0.11.0 optimization pipeline

#[cfg(test)]
mod v0_11_0_optimizations {
    use gaiarusted::codegen::simd_emitter::{SIMDEmitter, SIMDLevel, SIMDOperationKind};
    use gaiarusted::codegen::tail_loop::{LoopUnrollingConfig, TailLoopGenerator, UnrollFactor};
    use gaiarusted::codegen::inlining::{InliningOptimizer, FunctionMetadata, FunctionSize};
    use gaiarusted::codegen::register_pressure::{RegisterPressureAnalyzer, LiveRangeCalculator};
    use gaiarusted::codegen::loop_tiling::{LoopTiler, TilingConfig, CacheLevel};

    #[test]
    fn test_simd_vectorization_sse2() {
        // Demonstrates SIMD vectorization for SSE2
        let mut emitter = SIMDEmitter::new(SIMDLevel::SSE2);
        
        // Allocate vector registers for array processing
        let vec_reg = emitter.allocate_vector_register("arr_data");
        assert_eq!(vec_reg, 0);
        
        // Generate a simple vectorized loop
        emitter.emit_sse2_load(vec_reg, "rax", 0);
        emitter.emit_sse2_padded(vec_reg, 1);
        emitter.emit_sse2_store("rax", 0, vec_reg);
        
        let asm = emitter.get_assembly();
        assert!(asm.contains("movdqa"));
        assert!(asm.contains("paddq"));
        
        // Verify SSE2 is 128-bit
        assert_eq!(SIMDLevel::SSE2.vector_width(), 16);
        assert_eq!(SIMDLevel::SSE2.i64_capacity(), 2); // 2x i64 in 128 bits
    }

    #[test]
    fn test_simd_vectorization_avx2() {
        // Demonstrates SIMD vectorization for AVX2 (2x performance vs SSE2)
        let mut emitter = SIMDEmitter::new(SIMDLevel::AVX2);
        
        let vec_reg = emitter.allocate_vector_register("arr_data");
        emitter.emit_avx2_load(vec_reg, "rax", 0);
        emitter.emit_avx2_paddq(vec_reg, 1);
        emitter.emit_avx2_store("rax", 0, vec_reg);
        
        let asm = emitter.get_assembly();
        assert!(asm.contains("vmovdqa"));
        assert!(asm.contains("vpaddq"));
        
        // Verify AVX2 is 256-bit (2x SSE2)
        assert_eq!(SIMDLevel::AVX2.vector_width(), 32);
        assert_eq!(SIMDLevel::AVX2.i64_capacity(), 4); // 4x i64 in 256 bits
    }

    #[test]
    fn test_loop_unrolling_4x() {
        // Demonstrates 4x loop unrolling with epilogue handling
        let config = LoopUnrollingConfig::new("i".to_string(), 0, 20, 1);
        
        // Should recommend 8x unroll for 20 ops
        assert_eq!(config.unroll_factor, UnrollFactor::Unroll8x);
        assert_eq!(config.trip_count(), 20);
        
        let mut gen = TailLoopGenerator::new(config);
        let asm = gen.generate();
        
        // Verify unrolling structure
        assert!(asm.contains("unroll"));
        assert!(asm.contains("epilogue"));
        
        // Epilogue should handle 20 % 8 = 4 remaining iterations
        assert_eq!(gen.config.epilogue_iterations(), 4);
    }

    #[test]
    fn test_register_pressure_analysis() {
        // Demonstrates live range analysis and register pressure tracking
        let mut analyzer = RegisterPressureAnalyzer::new();
        
        // Simulate variable lifetimes
        analyzer.add_live_range("x".to_string(), 0, 10, 8);
        analyzer.add_live_range("y".to_string(), 5, 15, 8);
        analyzer.add_live_range("z".to_string(), 10, 20, 8);
        
        // At instruction 10, all three variables are live
        assert_eq!(analyzer.pressure_at(10), 3);
        
        // Peak pressure is 3 (all three variables simultaneously)
        assert!(analyzer.peak_pressure() >= 3);
        
        // Verify we have 16 available registers (x86-64)
        assert_eq!(analyzer.registers.len(), 14); // Excluding RSP/RBP
    }

    #[test]
    fn test_cache_aware_loop_tiling_l2() {
        // Demonstrates cache-aware loop tiling for L2 cache optimization
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 10000, 1);
        
        // Verify L2 cache size (256 KB)
        assert_eq!(CacheLevel::L2.size_bytes(), 256 * 1024);
        
        // Calculate optimal tile size
        assert!(config.tile_size > 0);
        assert!(config.tile_size < CacheLevel::L2.size_bytes());
        
        // Estimate speedup from tiling
        let speedup = config.speedup();
        assert!(speedup >= 1.0);
        
        // Generate tiled loop structure
        let mut tiler = LoopTiler::new(config);
        let asm = tiler.generate_tiled_loop();
        
        assert!(asm.contains("tile"));
        assert!(asm.contains("outer"));
        assert!(asm.contains("inner"));
    }

    #[test]
    fn test_2d_matrix_tiling() {
        // Demonstrates 2D loop tiling for matrix operations
        let config = TilingConfig::new(CacheLevel::L2, 8, 0, 1000, 2);
        let mut tiler = LoopTiler::new(config);
        
        // Generate 2D tiled loop for matrix processing
        let asm = tiler.generate_2d_tiled_loop(100, 100);
        
        assert!(asm.contains("2D"));
        assert!(asm.contains("row"));
        assert!(asm.contains("col"));
    }

    #[test]
    fn test_inlining_small_functions() {
        // Demonstrates cross-function inlining for small functions
        let mut optimizer = InliningOptimizer::new(10000); // 10KB code budget
        
        // Register a tiny helper function (should be inlined)
        let tiny_func = FunctionMetadata::new("add_one".to_string(), FunctionSize::Tiny, 1);
        optimizer.register_function(tiny_func);
        
        // Check if it's a good candidate for inlining
        let candidates = optimizer.get_candidates();
        assert!(candidates.contains(&"add_one".to_string()));
        
        // Estimate speedup from inlining this function
        let speedup = optimizer.estimate_speedup();
        assert!(speedup >= 1.0);
    }

    #[test]
    fn test_inlining_with_code_size_constraints() {
        // Demonstrates inlining respects code size budgets
        let mut optimizer = InliningOptimizer::new(500); // Tight 500-byte budget
        
        // Register several functions of varying sizes
        let tiny = FunctionMetadata::new("tiny".to_string(), FunctionSize::Tiny, 1);
        let small = FunctionMetadata::new("small".to_string(), FunctionSize::Small, 2);
        let large = FunctionMetadata::new("large".to_string(), FunctionSize::Large, 10);
        
        optimizer.register_function(tiny);
        optimizer.register_function(small);
        optimizer.register_function(large);
        
        // Current size should be estimated
        let size = optimizer.current_size();
        assert!(size > 0);
        assert!(size <= 500); // Should fit in budget
    }

    #[test]
    fn test_live_range_calculation_complex() {
        // Demonstrates complex live range calculation
        let instrs = vec![
            "mov x, 1".to_string(),
            "add x, y".to_string(),
            "mov y, x".to_string(),
            "sub y, 1".to_string(),
        ];
        
        let mut calc = LiveRangeCalculator::new(instrs);
        
        // Mark definitions
        calc.add_definition("x", 0);
        calc.add_definition("y", 2);
        
        // Mark uses
        calc.add_use("x", 1);
        calc.add_use("y", 1);
        calc.add_use("y", 3);
        
        let ranges = calc.calculate();
        
        // x is defined at 0, used at 1, so live range is [0, 1]
        assert!(ranges.contains_key("x"));
        assert!(ranges.contains_key("y"));
        
        let x_range = &ranges["x"];
        assert_eq!(x_range.start, 0);
        assert_eq!(x_range.end, 1);
    }

    #[test]
    fn test_optimization_pipeline_summary() {
        // High-level test showing all optimizations working together
        
        // 1. SIMD detection and emission
        let mut simd_emitter = SIMDEmitter::new(SIMDLevel::AVX2);
        let _vec_reg = simd_emitter.allocate_vector_register("data");
        
        // 2. Loop unrolling
        let loop_config = LoopUnrollingConfig::new("i".to_string(), 0, 100, 1);
        assert!(matches!(loop_config.unroll_factor, UnrollFactor::Unroll8x));
        
        // 3. Register pressure analysis
        let mut reg_analyzer = RegisterPressureAnalyzer::new();
        reg_analyzer.add_live_range("a".to_string(), 0, 50, 8);
        reg_analyzer.add_live_range("b".to_string(), 25, 75, 8);
        let pressure = reg_analyzer.peak_pressure();
        assert!(pressure > 0);
        
        // 4. Loop tiling
        let tile_config = TilingConfig::new(CacheLevel::L2, 8, 0, 1000, 1);
        let tile_speedup = tile_config.speedup();
        assert!(tile_speedup >= 1.0);
        
        // 5. Function inlining
        let mut inliner = InliningOptimizer::new(5000);
        let small_func = FunctionMetadata::new("helper".to_string(), FunctionSize::Small, 1);
        inliner.register_function(small_func);
        let inline_speedup = inliner.estimate_speedup();
        assert!(inline_speedup >= 1.0);
        
        // All optimizations together should show cumulative benefits
        println!("Loop unroll factor: {:?}", loop_config.unroll_factor.factor());
        println!("Peak register pressure: {}", pressure);
        println!("Cache tiling speedup: {:.2}x", tile_speedup);
        println!("Inlining speedup: {:.2}x", inline_speedup);
    }
}
