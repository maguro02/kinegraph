import { useEffect } from 'react';
import { Provider } from 'jotai';
import { useAtom } from 'jotai';
import { projectAtom } from './store/atoms';
import { Canvas } from './components/Canvas';
import { Timeline } from './components/Timeline';
import { Button } from './components/Button';
import { Toolbar } from './components/Toolbar';
import { LayerPanel } from './components/LayerPanel';

function AppContent() {
  const [project, setProject] = useAtom(projectAtom);

  useEffect(() => {
    // ハイブリッドシステムでは、プロジェクト管理はRust側で行う
    // 初期状態を取得
    const initProject = () => {
      try {
        // プロジェクトのデフォルトデータを設定
        if (!project) {
          setProject({
            id: 'project-1',
            name: '新規プロジェクト',
            width: 1920,
            height: 1080,
            frameRate: 24,
            frames: [
              {
                id: 'frame-1',
                duration: 1,
                layers: [
                  {
                    id: 'layer-1',
                    name: 'レイヤー 1',
                    visible: true,
                    opacity: 1,
                    blendMode: 'Normal' as const,
                    locked: false,
                  }
                ]
              }
            ]
          });
        }
      } catch (error) {
        console.error('プロジェクトの初期化に失敗しました:', error);
      }
    };

    initProject();
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
    <div className="flex flex-col h-screen bg-secondary-900 text-secondary-100">
      {/* ヘッダー */}
      <header className="bg-secondary-800 p-4 shadow-lg">
        <h1 className="text-2xl font-bold">Kinegraph - {project.name}</h1>
      </header>

      {/* メインコンテンツ */}
      <div className="flex flex-1 overflow-hidden">
        {/* 左サイドバー（ツールパネル） */}
        <aside className="w-64 bg-secondary-800 p-4 overflow-y-auto">
          <Toolbar />
          <div className="mt-8">
            <h2 className="text-lg font-semibold mb-4">プロジェクト情報</h2>
            <div className="space-y-2 text-sm">
              <div>サイズ: {project.width} × {project.height}</div>
              <div>フレームレート: {project.frameRate} fps</div>
            </div>
          </div>
        </aside>

        {/* 中央（キャンバス） */}
        <main className="flex-1 p-4 overflow-hidden">
          <div className="flex flex-col items-center justify-center h-full">
            <div className="mb-4">
              <Canvas 
                width={Math.min(project.width, 1920)} 
                height={Math.min(project.height, 1080)} 
              />
            </div>
          </div>
        </main>

        {/* 右サイドバー（レイヤーパネル） */}
        <aside className="w-64 bg-secondary-800 p-4 overflow-y-auto">
          <LayerPanel />
          <div className="mt-4 space-y-2">
            <Button 
              variant="secondary" 
              size="sm" 
              className="w-full"
              onClick={() => {
                // TODO: Undo実装
                console.log('Undo');
              }}
            >
              Undo
            </Button>
            <Button 
              variant="secondary" 
              size="sm" 
              className="w-full"
              onClick={() => {
                // TODO: Redo実装
                console.log('Redo');
              }}
            >
              Redo
            </Button>
          </div>
        </aside>
      </div>

      {/* 下部（タイムライン） */}
      <footer className="h-48 bg-secondary-800 border-t border-secondary-700">
        <Timeline />
      </footer>
      
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