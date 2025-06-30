use std::sync::Arc;
use tokio::sync::Mutex;
use kinegraph_lib::drawing_engine::DrawingEngine;
use kinegraph_lib::animation::Project;
use kinegraph_lib::api::CreateProjectArgs;

/// 共通テストヘルパー関数
pub fn create_test_state() -> Arc<Mutex<DrawingEngine>> {
    Arc::new(Mutex::new(DrawingEngine::new()))
}

/// テスト用プロジェクト作成ヘルパー
pub fn create_test_project(name: &str, width: u32, height: u32, frame_rate: f32) -> Project {
    Project::new(name.to_string(), width, height, frame_rate)
}

/// テスト用CreateProjectArgs作成ヘルパー
pub fn create_test_project_args(name: &str, width: u32, height: u32, frame_rate: f32) -> CreateProjectArgs {
    CreateProjectArgs {
        name: name.to_string(),
        width,
        height,
        frame_rate,
    }
}

/// 標準的なテストプロジェクト仕様
pub mod test_specs {
    pub const DEFAULT_WIDTH: u32 = 1920;
    pub const DEFAULT_HEIGHT: u32 = 1080;
    pub const DEFAULT_FRAME_RATE: f32 = 24.0;
    
    pub const HD_WIDTH: u32 = 1280;
    pub const HD_HEIGHT: u32 = 720;
    
    pub const UHD_WIDTH: u32 = 3840;
    pub const UHD_HEIGHT: u32 = 2160;
    pub const HIGH_FRAME_RATE: f32 = 60.0;
}

/// 共通アサーション関数
pub mod assertions {
    use super::*;
    
    pub fn assert_drawing_engine_initial_state(engine: &DrawingEngine) {
        assert!(engine.surface.is_none());
        assert!(engine.adapter.is_none());
        assert!(engine.device.is_none());
        assert!(engine.queue.is_none());
    }
    
    pub fn assert_project_properties(project: &Project, name: &str, width: u32, height: u32, frame_rate: f32) {
        assert_eq!(project.name, name);
        assert_eq!(project.width, width);
        assert_eq!(project.height, height);
        assert_eq!(project.frame_rate, frame_rate);
        // プロジェクトは初期フレームを1つ持つ
        assert_eq!(project.frames.len(), 1);
        // 初期フレームの確認
        let initial_frame = &project.frames[0];
        assert!(initial_frame.id.starts_with("frame_"));
        assert!(initial_frame.layers.is_empty());
        assert_eq!(initial_frame.duration, 1.0 / frame_rate);
    }
    
    pub fn assert_create_project_args(args: &CreateProjectArgs, name: &str, width: u32, height: u32, frame_rate: f32) {
        assert_eq!(args.name, name);
        assert_eq!(args.width, width);
        assert_eq!(args.height, height);
        assert_eq!(args.frame_rate, frame_rate);
    }
}