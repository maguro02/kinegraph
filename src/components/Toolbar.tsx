import { useAtom } from 'jotai';
import { toolAtom, brushSizeAtom, colorAtom, jointTypeAtom, drawingEngineAtom } from '../store/atoms';
import { Button } from './Button';

// アイコンコンポーネント（簡易版）
const PenIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
  </svg>
);

const EraserIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
  </svg>
);

const BucketIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
  </svg>
);

const SelectIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
  </svg>
);

export function Toolbar() {
  const [currentTool, setCurrentTool] = useAtom(toolAtom);
  const [brushSize, setBrushSize] = useAtom(brushSizeAtom);
  const [color, setColor] = useAtom(colorAtom);
  const [jointType, setJointType] = useAtom(jointTypeAtom);
  const [drawingEngine, setDrawingEngine] = useAtom(drawingEngineAtom);

  const tools = [
    { id: 'pen' as const, icon: <PenIcon />, label: 'ペン' },
    { id: 'eraser' as const, icon: <EraserIcon />, label: '消しゴム' },
    { id: 'bucket' as const, icon: <BucketIcon />, label: '塗りつぶし' },
    { id: 'select' as const, icon: <SelectIcon />, label: '選択' },
  ];

  return (
    <div className="fixed left-4 top-20 z-50 bg-secondary-800 border border-secondary-700 rounded-xl shadow-lg p-3">
      <div className="flex flex-col gap-4">
        {/* ツール選択 */}
        <div className="flex flex-col gap-2">
          {tools.map((tool) => (
            <Button
              key={tool.id}
              variant="icon"
              onClick={() => setCurrentTool(tool.id)}
              className={currentTool === tool.id ? 'bg-primary-600 text-white' : ''}
              icon={tool.icon}
            />
          ))}
        </div>

        {/* 区切り線 */}
        <div className="border-t border-secondary-700"></div>

        {/* ブラシサイズ */}
        <div className="flex flex-col gap-2 items-center">
          <label className="text-xs text-secondary-300">サイズ</label>
          <div className="flex flex-col items-center gap-1">
            <input
              type="range"
              min="1"
              max="50"
              value={brushSize}
              onChange={(e) => setBrushSize(Number(e.target.value))}
              className="w-16 h-2 bg-secondary-600 rounded-lg appearance-none cursor-pointer [writing-mode:bt-lr] transform rotate-180"
              style={{ transform: 'rotate(-90deg)' }}
            />
            <span className="text-xs text-secondary-300 min-w-[2rem] text-center">{brushSize}</span>
          </div>
        </div>

        {/* 区切り線 */}
        <div className="border-t border-secondary-700"></div>

        {/* カラーピッカー */}
        <div className="flex flex-col gap-2 items-center">
          <label className="text-xs text-secondary-300">色</label>
          <input
            type="color"
            value={color}
            onChange={(e) => setColor(e.target.value)}
            className="w-8 h-8 rounded-lg border border-secondary-600 cursor-pointer"
          />
        </div>

        {/* 区切り線 */}
        <div className="border-t border-secondary-700"></div>

        {/* ジョイントタイプ選択 */}
        <div className="flex flex-col gap-2">
          <label className="text-xs text-secondary-300 text-center">線の接続</label>
          <div className="flex flex-col gap-1">
            <button
              onClick={() => setJointType({ type: 'round', segments: 8 })}
              className={`px-2 py-1 text-xs rounded ${
                jointType.type === 'round' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              }`}
              title="滑らかな接続"
            >
              ラウンド
            </button>
            <button
              onClick={() => setJointType({ type: 'bevel' })}
              className={`px-2 py-1 text-xs rounded ${
                jointType.type === 'bevel' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              }`}
              title="面取り接続"
            >
              ベベル
            </button>
            <button
              onClick={() => setJointType({ type: 'miter', limit: 4.0 })}
              className={`px-2 py-1 text-xs rounded ${
                jointType.type === 'miter' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              }`}
              title="鋭角的な接続"
            >
              マイター
            </button>
          </div>
          
          {/* ラウンドジョイントのセグメント数調整 */}
          {jointType.type === 'round' && (
            <div className="flex flex-col gap-1 mt-2">
              <label className="text-xs text-secondary-400">滑らかさ</label>
              <input
                type="range"
                min="3"
                max="16"
                value={jointType.segments}
                onChange={(e) => setJointType({ type: 'round', segments: Number(e.target.value) })}
                className="w-full h-2 bg-secondary-600 rounded-lg appearance-none cursor-pointer"
              />
              <span className="text-xs text-secondary-400 text-center">{jointType.segments}</span>
            </div>
          )}
          
          {/* マイタージョイントのリミット調整 */}
          {jointType.type === 'miter' && (
            <div className="flex flex-col gap-1 mt-2">
              <label className="text-xs text-secondary-400">リミット</label>
              <input
                type="range"
                min="1"
                max="10"
                step="0.5"
                value={jointType.limit}
                onChange={(e) => setJointType({ type: 'miter', limit: Number(e.target.value) })}
                className="w-full h-2 bg-secondary-600 rounded-lg appearance-none cursor-pointer"
              />
              <span className="text-xs text-secondary-400 text-center">{jointType.limit.toFixed(1)}</span>
            </div>
          )}
        </div>
        
        {/* 区切り線 */}
        <div className="border-t border-secondary-700"></div>
        
        {/* 描画エンジン選択（開発時のみ） */}
        {process.env.NODE_ENV === 'development' && (
          <div className="flex flex-col gap-2">
            <label className="text-xs text-secondary-300 text-center">描画エンジン</label>
            <div className="flex flex-col gap-1">
              <button
                onClick={() => setDrawingEngine('canvas2d')}
                className={`px-2 py-1 text-xs rounded ${
                  drawingEngine === 'canvas2d' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
                }`}
                title="Canvas 2D API"
              >
                Canvas2D
              </button>
              <button
                onClick={() => setDrawingEngine('wasm')}
                className={`px-2 py-1 text-xs rounded ${
                  drawingEngine === 'wasm' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
                }`}
                title="WASM ダイレクト"
              >
                WASM
              </button>
              <button
                onClick={() => setDrawingEngine('wasmWorker')}
                className={`px-2 py-1 text-xs rounded ${
                  drawingEngine === 'wasmWorker' ? 'bg-primary-600 text-white' : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
                }`}
                title="WASM Worker"
              >
                WASM Worker
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}