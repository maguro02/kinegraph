import { useAtom } from 'jotai';
import { projectAtom, currentFrameIndexAtom } from '../store/atoms';
import { Panel } from './Panel';
import { Button } from './Button';

// アイコンコンポーネント
const PlayIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1m-6 4h8m2-4a9 9 0 11-18 0 9 9 0 0118 0z" />
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

export function Timeline() {
  const [project] = useAtom(projectAtom);
  const [currentFrameIndex, setCurrentFrameIndex] = useAtom(currentFrameIndexAtom);

  const frames = project?.frames || [];

  return (
    <Panel
      title="タイムライン"
      className="h-48"
      headerActions={
        <>
          <Button variant="icon" icon={<PlayIcon />} />
          <Button variant="icon" icon={<PlusIcon />} />
          <Button variant="icon" icon={<TrashIcon />} />
        </>
      }
    >
      <div className="flex flex-col gap-4">
        {/* フレームレート表示 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-secondary-300">
            フレーム: {currentFrameIndex + 1} / {frames.length || 1}
          </span>
          <span className="text-secondary-300">
            {project?.frameRate || 24} fps
          </span>
        </div>

        {/* フレーム一覧 */}
        <div className="flex gap-1 overflow-x-auto pb-2">
          {frames.length > 0 ? (
            frames.map((frame, index) => (
              <div
                key={frame.id}
                className={`frame-item ${index === currentFrameIndex ? 'current' : ''}`}
                onClick={() => setCurrentFrameIndex(index)}
              >
                <div className="frame-thumbnail bg-secondary-600">
                  {/* フレームサムネイル */}
                </div>
                <div className="absolute bottom-0 left-0 right-0 text-xs text-center bg-black/50 text-white py-0.5">
                  {index + 1}
                </div>
              </div>
            ))
          ) : (
            <div className="frame-item current">
              <div className="frame-thumbnail bg-secondary-600">
                {/* 空のフレーム */}
              </div>
              <div className="absolute bottom-0 left-0 right-0 text-xs text-center bg-black/50 text-white py-0.5">
                1
              </div>
            </div>
          )}
        </div>

        {/* オニオンスキン設定 */}
        <div className="flex items-center gap-2 text-sm">
          <label className="text-secondary-300">オニオンスキン:</label>
          <input
            type="checkbox"
            className="rounded"
          />
          <input
            type="range"
            min="0"
            max="100"
            defaultValue="50"
            className="w-16 h-2 bg-secondary-600 rounded-lg appearance-none cursor-pointer"
          />
        </div>
      </div>
    </Panel>
  );
}