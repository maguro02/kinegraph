import { useEffect } from 'react';
import { Provider } from 'jotai';
import { useAtom } from 'jotai';
import { projectAtom } from './store/atoms';
import { createProject, initializeDebugLogging } from './lib/tauri';
import { Toolbar } from './components/Toolbar';
import { Canvas } from './components/Canvas';
import { LayerPanel } from './components/LayerPanel';
import { Timeline } from './components/Timeline';
import { Button } from './components/Button';

function AppContent() {
  const [project, setProject] = useAtom(projectAtom);

  useEffect(() => {
    // デバッグログ初期化
    const initDebug = async () => {
      try {
        await initializeDebugLogging();
      } catch (error) {
        console.error('デバッグログ初期化に失敗しました:', error);
      }
    };

    // デモ用のプロジェクトを作成
    const initProject = async () => {
      try {
        const newProject = await createProject('サンプルプロジェクト', 1920, 1080, 24);
        setProject(newProject);
      } catch (error) {
        console.error('プロジェクトの作成に失敗しました:', error);
      }
    };

    if (!project) {
      // 順番にデバッグ初期化してからプロジェクト作成
      initDebug().then(() => {
        initProject();
      });
    }
  }, [project, setProject]);

  if (!project) {
    return (
      <div className="flex items-center justify-center h-screen bg-secondary-900">
        <div className="text-center">
          <div className="text-2xl font-bold text-secondary-100 mb-4">Kinegraph</div>
          <div className="text-secondary-400">プロジェクトを初期化中...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen bg-secondary-900 flex flex-col">
      {/* メニューバー */}
      <div className="bg-secondary-800 border-b border-secondary-700 px-4 py-2 flex items-center justify-between">
        <div className="text-lg font-semibold text-secondary-100">
          Kinegraph - {project.name}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm">ファイル</Button>
          <Button variant="ghost" size="sm">編集</Button>
          <Button variant="ghost" size="sm">表示</Button>
          <Button variant="ghost" size="sm">ヘルプ</Button>
        </div>
      </div>

      {/* ツールバー */}
      <Toolbar />

      {/* メインエリア */}
      <div className="flex flex-1 overflow-hidden">
        {/* レイヤーパネル */}
        <div className="bg-secondary-800 border-r border-secondary-700">
          <LayerPanel />
        </div>

        {/* キャンバスエリア */}
        <div className="flex-1 flex flex-col">
          <Canvas width={project.width} height={project.height} />
        </div>

        {/* 右サイドパネル（将来的にプロパティパネルなど） */}
        <div className="w-64 bg-secondary-800 border-l border-secondary-700 p-4">
          <div className="text-sm text-secondary-300 mb-4">プロパティ</div>
          <div className="space-y-2 text-sm text-secondary-400">
            <div>キャンバス: {project.width} × {project.height}</div>
            <div>フレームレート: {project.frameRate} fps</div>
          </div>
        </div>
      </div>

      {/* タイムライン */}
      <div className="bg-secondary-800 border-t border-secondary-700">
        <Timeline />
      </div>
    </div>
  );
}

function App() {
  return (
    <Provider>
      <AppContent />
    </Provider>
  );
}

export default App;
