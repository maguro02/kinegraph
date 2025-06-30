use kinegraph_lib::drawing_engine::DrawingEngine;
use log::{info, debug};

#[tokio::test]
async fn test_offscreen_renderer_creation() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    
    info!("[TEST] オフスクリーンレンダラー作成テスト開始");
    
    // DrawingEngine を初期化
    let mut engine = DrawingEngine::new();
    engine.initialize().await.expect("DrawingEngine 初期化に失敗");
    
    // 小さなサイズでオフスクリーンレンダラーを作成
    let renderer = engine.create_offscreen_renderer(800, 600);
    assert!(renderer.is_ok(), "オフスクリーンレンダラー作成に失敗: {:?}", renderer.err());
    
    let renderer = renderer.unwrap();
    assert_eq!(renderer.dimensions(), (800, 600));
    
    info!("[TEST] オフスクリーンレンダラー作成テスト成功");
}

#[tokio::test]
async fn test_offscreen_rendering() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    
    info!("[TEST] オフスクリーンレンダリング実行テスト開始");
    
    // DrawingEngine を初期化
    let mut engine = DrawingEngine::new();
    engine.initialize().await.expect("DrawingEngine 初期化に失敗");
    
    // オフスクリーンレンダラーを作成
    let renderer = engine.create_offscreen_renderer(100, 100)
        .expect("オフスクリーンレンダラー作成に失敗");
    
    // レンダリングを実行
    let pixel_data = engine.render_offscreen(&renderer).await;
    assert!(pixel_data.is_ok(), "オフスクリーンレンダリングに失敗: {:?}", pixel_data.err());
    
    let pixel_data = pixel_data.unwrap();
    
    // 実際のデータサイズを取得（アライメントされた値）
    let expected_min_size = 100 * 100 * 4; // 最小サイズ（40,000バイト）
    assert!(pixel_data.len() >= expected_min_size, 
            "データサイズが小さすぎます: {} < {}", pixel_data.len(), expected_min_size);
    
    debug!("[TEST] 実際のデータサイズ: {} バイト", pixel_data.len());
    debug!("[TEST] 最初のピクセル: [{}, {}, {}, {}]", 
           pixel_data[0], pixel_data[1], pixel_data[2], pixel_data[3]);
    
    // 最初のピクセルが白色であることを確認
    assert_eq!(pixel_data[0], 255, "R成分が正しくありません");
    assert_eq!(pixel_data[1], 255, "G成分が正しくありません");
    assert_eq!(pixel_data[2], 255, "B成分が正しくありません");
    assert_eq!(pixel_data[3], 255, "A成分が正しくありません");
    
    info!("[TEST] オフスクリーンレンダリング実行テスト成功");
}

#[tokio::test]
async fn test_invalid_dimensions() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    
    info!("[TEST] 無効な寸法テスト開始");
    
    let mut engine = DrawingEngine::new();
    engine.initialize().await.expect("DrawingEngine 初期化に失敗");
    
    // 無効な寸法（0x0）
    let result = engine.create_offscreen_renderer(0, 0);
    assert!(result.is_err(), "無効な寸法(0x0)が受け入れられました");
    
    // 大きすぎる寸法（4K以上）
    let result = engine.create_offscreen_renderer(4000, 3000);
    assert!(result.is_err(), "大きすぎる寸法が受け入れられました");
    
    info!("[TEST] 無効な寸法テスト成功");
}

#[tokio::test]
async fn test_4k_resolution() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    
    info!("[TEST] 4K解像度テスト開始");
    
    let mut engine = DrawingEngine::new();
    engine.initialize().await.expect("DrawingEngine 初期化に失敗");
    
    // 4K解像度（3840x2160）
    let renderer = engine.create_offscreen_renderer(3840, 2160);
    assert!(renderer.is_ok(), "4K解像度での作成に失敗: {:?}", renderer.err());
    
    let renderer = renderer.unwrap();
    assert_eq!(renderer.dimensions(), (3840, 2160));
    
    info!("[TEST] 4K解像度テスト成功（レンダリングは時間がかかるためスキップ）");
}

#[test]
fn test_renderer_resize() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();
    
    info!("[TEST] レンダラーリサイズテスト開始");
    
    let mut renderer = kinegraph_lib::drawing_engine::OffscreenRenderer::new(800, 600)
        .expect("レンダラー作成に失敗");
    
    assert_eq!(renderer.dimensions(), (800, 600));
    
    // リサイズ
    let result = renderer.resize(1024, 768);
    assert!(result.is_ok(), "リサイズに失敗: {:?}", result.err());
    assert_eq!(renderer.dimensions(), (1024, 768));
    
    // 無効な寸法でリサイズ
    let result = renderer.resize(0, 100);
    assert!(result.is_err(), "無効な寸法でのリサイズが受け入れられました");
    
    info!("[TEST] レンダラーリサイズテスト成功");
}