import { useEffect } from 'react';
import { Provider } from 'jotai';
import { useAtom } from 'jotai';
import { projectAtom } from './store/atoms';
import { createProject, initializeDebugLogging } from './lib/tauri';
import { Canvas } from './components/Canvas';
import { Timeline } from './components/Timeline';
import { Button } from './components/Button';
import { Toolbar } from './components/Toolbar';
import { LayerPanel } from './components/LayerPanel';

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
    <div className="h-screen bg-secondary-900 relative overflow-hidden">
      {/* メニューバー - 上部固定 */}
      <div className="absolute top-0 left-0 right-0 h-12 z-50 bg-secondary-800 border-b border-secondary-700 px-4 py-2 flex items-center justify-between">
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

      {/* キャンバスエリア - 全画面表示（メニューバーとタイムラインを除く） */}
      <div className="absolute top-12 left-0 right-0 bottom-32 bg-secondary-900">
        <Canvas width={project.width} height={project.height} />
      </div>

      {/* タイムライン - 下部固定 */}
      <div className="absolute bottom-0 left-0 right-0 h-32 z-50 bg-secondary-800 border-t border-secondary-700">
        <Timeline />
      </div>

      {/* ツールバー - 左側固定 */}
      <Toolbar />

      {/* レイヤーパネル - 右側固定 */}
      <LayerPanel />
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
