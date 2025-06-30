use super::*;

/// パイプラインの統合テスト
async fn create_test_environment() -> Result<(DrawingEngine, (u32, u32)), Box<dyn std::error::Error>> {
    let mut engine = DrawingEngine::new();
    engine.initialize().await?;
    
    let canvas_size = (512, 512);
    
    // テスト用レイヤーテクスチャを作成
    engine.create_layer_texture("test_layer", canvas_size.0, canvas_size.1)?;
    
    Ok((engine, canvas_size))
}

#[tokio::test]
async fn test_draw_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let (engine, canvas_size) = create_test_environment().await?;
    
    // 赤い線を描画（左上から右下へ）
    let start = engine.screen_to_normalized((50.0, 50.0), canvas_size);
    let end = engine.screen_to_normalized((450.0, 450.0), canvas_size);
    let red_color = [1.0, 0.0, 0.0, 1.0]; // 赤色
    let line_width = 3.0;
    
    engine.draw_line_to_layer("test_layer", start, end, red_color, line_width)?;
    
    // レイヤーの内容を取得して確認
    let pixel_data = engine.get_layer_texture_data("test_layer").await?;
    assert!(!pixel_data.is_empty());
    assert_eq!(pixel_data.len(), 512 * 512 * 4); // RGBA8
    
    // 最初の数ピクセルが透明（背景）であることを確認
    let first_pixel = &pixel_data[0..4];
    assert_eq!(first_pixel, [0, 0, 0, 0]); // 透明
    
    println!("✓ 単一線描画テスト成功");
    Ok(())
}

#[tokio::test]
async fn test_draw_stroke_with_pressure() -> Result<(), Box<dyn std::error::Error>> {
    let (engine, canvas_size) = create_test_environment().await?;
    
    // 筆圧変化のあるストロークを作成
    let mut stroke = DrawStroke::new([0.0, 1.0, 0.0, 1.0], 5.0); // 緑色、基本幅5px
    
    // 曲線的なストロークを追加（筆圧変化付き）
    for i in 0..20 {
        let t = i as f32 / 19.0; // 0.0 ～ 1.0
        let x = 100.0 + t * 300.0; // 100px ～ 400px
        let y = 200.0 + (t * std::f32::consts::PI * 2.0).sin() * 50.0; // サイン波
        let pressure = 0.3 + 0.7 * (t * std::f32::consts::PI).sin().abs(); // 筆圧変化
        
        let norm_pos = engine.screen_to_normalized((x, y), canvas_size);
        stroke.add_point(norm_pos.0, norm_pos.1, pressure);
    }
    
    engine.draw_stroke_to_layer("test_layer", &stroke)?;
    
    // レイヤーの内容を取得
    let pixel_data = engine.get_layer_texture_data("test_layer").await?;
    assert!(!pixel_data.is_empty());
    
    println!("✓ 筆圧変化ストローク描画テスト成功");
    Ok(())
}

#[tokio::test]
async fn test_multiple_overlapping_strokes() -> Result<(), Box<dyn std::error::Error>> {
    let (engine, canvas_size) = create_test_environment().await?;
    
    // 複数の重なり合うストロークを描画
    let colors = [
        [1.0, 0.0, 0.0, 0.7], // 半透明赤
        [0.0, 1.0, 0.0, 0.7], // 半透明緑
        [0.0, 0.0, 1.0, 0.7], // 半透明青
    ];
    
    for (i, color) in colors.iter().enumerate() {
        let mut stroke = DrawStroke::new(*color, 4.0);
        
        // 各色で異なる方向の線を描画
        let angle = i as f32 * std::f32::consts::PI * 2.0 / 3.0;
        let center = (256.0, 256.0);
        let radius = 150.0;
        
        for j in 0..10 {
            let t = j as f32 / 9.0;
            let x = center.0 + (angle + t * std::f32::consts::PI).cos() * radius * t;
            let y = center.1 + (angle + t * std::f32::consts::PI).sin() * radius * t;
            
            let norm_pos = engine.screen_to_normalized((x, y), canvas_size);
            stroke.add_point(norm_pos.0, norm_pos.1, 1.0);
        }
        
        engine.draw_stroke_to_layer("test_layer", &stroke)?;
    }
    
    // レイヤーの内容を取得
    let pixel_data = engine.get_layer_texture_data("test_layer").await?;
    assert!(!pixel_data.is_empty());
    
    println!("✓ 複数重なりストローク描画テスト成功");
    Ok(())
}

#[tokio::test]
async fn test_coordinate_conversion_accuracy() -> Result<(), Box<dyn std::error::Error>> {
    let engine = DrawingEngine::new();
    let canvas_size = (800, 600);
    
    // 座標変換の精度テスト
    let test_points = vec![
        (0.0, 0.0),       // 左上
        (400.0, 300.0),   // 中央
        (800.0, 600.0),   // 右下
        (200.0, 150.0),   // 任意の点
    ];
    
    for (x, y) in test_points {
        let norm = engine.screen_to_normalized((x, y), canvas_size);
        let back = engine.normalized_to_screen(norm, canvas_size);
        
        // 許容誤差内での変換確認
        assert!((back.0 - x).abs() < 1e-3, "X座標変換誤差: {} -> {} -> {}", x, norm.0, back.0);
        assert!((back.1 - y).abs() < 1e-3, "Y座標変換誤差: {} -> {} -> {}", y, norm.1, back.1);
    }
    
    println!("✓ 座標変換精度テスト成功");
    Ok(())
}

#[tokio::test]
async fn test_clear_and_redraw() -> Result<(), Box<dyn std::error::Error>> {
    let (mut engine, canvas_size) = create_test_environment().await?;
    
    // 最初に線を描画
    let start = engine.screen_to_normalized((100.0, 100.0), canvas_size);
    let end = engine.screen_to_normalized((400.0, 400.0), canvas_size);
    engine.draw_line_to_layer("test_layer", start, end, [1.0, 0.0, 0.0, 1.0], 2.0)?;
    
    // レイヤーをクリア
    engine.clear_layer_texture("test_layer", None)?; // 透明でクリア
    
    // 新しい線を描画
    let start2 = engine.screen_to_normalized((400.0, 100.0), canvas_size);
    let end2 = engine.screen_to_normalized((100.0, 400.0), canvas_size);
    engine.draw_line_to_layer("test_layer", start2, end2, [0.0, 1.0, 0.0, 1.0], 3.0)?;
    
    // 結果を確認
    let pixel_data = engine.get_layer_texture_data("test_layer").await?;
    assert!(!pixel_data.is_empty());
    
    println!("✓ クリア・再描画テスト成功");
    Ok(())
}

/// パフォーマンステスト：大量のストローク描画
#[tokio::test]
async fn test_performance_many_strokes() -> Result<(), Box<dyn std::error::Error>> {
    let (engine, canvas_size) = create_test_environment().await?;
    
    let start_time = std::time::Instant::now();
    
    // 100本のランダムなストロークを描画
    for i in 0..100 {
        let mut stroke = DrawStroke::new(
            [
                (i % 256) as f32 / 255.0,
                ((i * 2) % 256) as f32 / 255.0,
                ((i * 3) % 256) as f32 / 255.0,
                0.8,
            ],
            2.0,
        );
        
        // 各ストロークに5-15個の点を追加
        let point_count = 5 + (i % 10);
        for j in 0..point_count {
            let t = j as f32 / (point_count - 1) as f32;
            let x = 50.0 + (i % 10) as f32 * 40.0 + t * 100.0;
            let y = 50.0 + (i / 10) as f32 * 40.0 + (t * std::f32::consts::PI * 4.0).sin() * 20.0;
            
            let norm_pos = engine.screen_to_normalized((x, y), canvas_size);
            stroke.add_point(norm_pos.0, norm_pos.1, 1.0);
        }
        
        engine.draw_stroke_to_layer("test_layer", &stroke)?;
    }
    
    let elapsed = start_time.elapsed();
    println!("✓ パフォーマンステスト成功: 100ストローク描画 {}ms", elapsed.as_millis());
    
    // 10秒以内で完了することを確認
    assert!(elapsed.as_secs() < 10, "描画が遅すぎます: {}秒", elapsed.as_secs());
    
    Ok(())
}

/// メモリ使用量テスト
#[tokio::test] 
async fn test_memory_usage() -> Result<(), Box<dyn std::error::Error>> {
    let (mut engine, _) = create_test_environment().await?;
    
    // 初期メモリ使用量を確認
    let (initial_memory, limit, active, total) = engine.get_texture_memory_stats()
        .ok_or("テクスチャメモリ統計が取得できません")?;
    
    assert!(initial_memory > 0, "初期メモリ使用量が0です");
    assert!(limit > initial_memory, "メモリ上限が現在使用量より小さいです");
    assert_eq!(active, 1, "アクティブテクスチャ数が予期された値と異なります");
    assert_eq!(total, 1, "総テクスチャ数が予期された値と異なります");
    
    println!("✓ メモリ使用量テスト成功: {}KB使用 / {}KB上限", 
        initial_memory / 1024, limit / 1024);
    
    // 複数レイヤー作成
    for i in 1..5 {
        engine.create_layer_texture(&format!("layer_{}", i), 256, 256)?;
    }
    
    let (after_memory, _, active_after, total_after) = engine.get_texture_memory_stats()
        .ok_or("テクスチャメモリ統計が取得できません")?;
    
    assert!(after_memory > initial_memory, "メモリ使用量が増加していません");
    assert_eq!(active_after, 5, "アクティブテクスチャ数が予期された値と異なります");
    assert_eq!(total_after, 5, "総テクスチャ数が予期された値と異なります");
    
    println!("✓ 複数レイヤーメモリテスト成功: {}KB使用", after_memory / 1024);
    
    Ok(())
}