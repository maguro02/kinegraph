import { useAtom } from 'jotai';
import { currentFrameAtom, selectedLayerAtom } from '../store/atoms';
import { Panel } from './Panel';
import { Button } from './Button';

// アイコンコンポーネント
const EyeIcon = ({ visible }: { visible: boolean }) => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    {visible ? (
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    ) : (
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L8.464 8.464M14.12 14.12l1.414 1.414" />
    )}
  </svg>
);

const LockIcon = ({ locked }: { locked: boolean }) => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    {locked ? (
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
    ) : (
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z" />
    )}
  </svg>
);

const PlusIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
  </svg>
);

const TrashIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
  </svg>
);

export function LayerPanel() {
  const [currentFrame] = useAtom(currentFrameAtom);
  const [selectedLayer, setSelectedLayer] = useAtom(selectedLayerAtom);

  const layers = currentFrame?.layers || [];

  return (
    <Panel
      title="レイヤー"
      className="w-64 h-full"
      headerActions={
        <>
          <Button variant="icon" icon={<PlusIcon />} />
          <Button variant="icon" icon={<TrashIcon />} />
        </>
      }
    >
      <div className="space-y-1">
        {layers.map((layer) => (
          <div
            key={layer.id}
            className={`layer-item ${selectedLayer === layer.id ? 'selected' : ''}`}
            onClick={() => setSelectedLayer(layer.id)}
          >
            <div className="flex items-center gap-2">
              <Button
                variant="icon"
                icon={<EyeIcon visible={layer.visible} />}
                className="p-1 h-6 w-6"
              />
              <Button
                variant="icon"
                icon={<LockIcon locked={layer.locked} />}
                className="p-1 h-6 w-6"
              />
            </div>
            
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-secondary-100 truncate">
                {layer.name}
              </div>
              <div className="text-xs text-secondary-400">
                {layer.blendMode} • {Math.round(layer.opacity * 100)}%
              </div>
            </div>
            
            <div className="w-12 h-8 bg-secondary-700 rounded border border-secondary-600">
              {/* レイヤーサムネイル */}
            </div>
          </div>
        ))}
        
        {layers.length === 0 && (
          <div className="text-center text-secondary-400 py-8">
            レイヤーがありません
          </div>
        )}
      </div>
    </Panel>
  );
}