import React from 'react';
import { useAtom } from 'jotai';
import { useState, useCallback, useEffect, useRef } from 'react';
import { projectAtom, currentFrameAtom, selectedLayerAtom, Layer } from '../store/atoms.ts';
import { Button } from './Button.tsx';
import { useDrawingEngine } from '../lib/useDrawingEngine.ts';

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

const EditIcon = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
  </svg>
);

// レイヤーサムネイル用のミニキャンバスコンポーネント
const LayerThumbnail = ({ layerId, drawingEngine }: { layerId: string; drawingEngine: any }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  
  useEffect(() => {
    const renderThumbnail = async () => {
      const canvas = canvasRef.current;
      if (!canvas || !layerId || !drawingEngine) return;
      
      try {
        await drawingEngine.renderToCanvas(canvas, layerId);
      } catch (error) {
        console.warn('レイヤーサムネイルの描画に失敗:', error);
      }
    };
    
    renderThumbnail();
  }, [layerId, drawingEngine]);
  
  return (
    <canvas 
      ref={canvasRef}
      className="w-12 h-8 bg-secondary-700 rounded border border-secondary-600 flex-shrink-0"
      style={{ imageRendering: 'pixelated' }}
    />
  );
};

export function LayerPanel() {
  const [project, setProject] = useAtom(projectAtom);
  const [currentFrame] = useAtom(currentFrameAtom);
  const [selectedLayer, setSelectedLayer] = useAtom(selectedLayerAtom);
  const [editingLayerId, setEditingLayerId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<string | null>(null);
  
  const { drawingEngine, isInitialized: isEngineInitialized, error: engineError, clearError } = useDrawingEngine();
  const [error, setError] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [debugInfo, setDebugInfo] = useState<{
    projectState?: any;
    layerCount?: number;
    engineState?: string;
    lastError?: string | null;
  }>({});

  const layers = currentFrame?.layers || [];
  const maxLayers = 10;
  const canAddLayer = layers.length < maxLayers;
  
  // 総合エラー状態
  const displayError = error || engineError;

  // レイヤー作成
  const handleCreateLayer = useCallback(async () => {
    console.log('🎨 レイヤー作成開始');
    
    // 詳細な前提条件チェック
    const preConditions = {
      hasProject: !!project,
      projectId: project?.id,
      projectDimensions: project ? `${project.width}x${project.height}` : 'N/A',
      hasCurrentFrame: !!currentFrame,
      frameId: currentFrame?.id,
      currentLayerCount: layers.length,
      maxLayers,
      canAddLayer,
      isEngineInitialized,
      hasDrawingEngine: !!drawingEngine
    };
    
    console.log('📊 レイヤー作成前の状態:', preConditions);
    setDebugInfo(prev => ({ ...prev, projectState: preConditions, layerCount: layers.length }));
    
    if (!project || !currentFrame || !canAddLayer) {
      const reason = !project ? 'プロジェクトなし' : 
                    !currentFrame ? 'フレームなし' : 
                    !canAddLayer ? `レイヤー上限(${layers.length}/${maxLayers})` : '不明';
      console.error('❌ レイヤー作成の前提条件未達成:', reason);
      setError(`レイヤーを作成できません: ${reason}`);
      return;
    }
    
    if (!isEngineInitialized) {
      console.error('❌ 描画エンジンが初期化されていません');
      setError('描画エンジンの初期化を待ってください');
      return;
    }
    
    setIsCreating(true);
    setError(null);
    
    try {
      const timestamp = Date.now();
      const newLayerId = `layer_${timestamp}`;
      
      console.log(`🔧 新しいレイヤーID生成: ${newLayerId}`);
      
      const newLayer: Layer = {
        id: newLayerId,
        name: `レイヤー ${layers.length + 1}`,
        visible: true,
        opacity: 1,
        blendMode: 'Normal',
        locked: false,
      };
      
      console.log('📦 新レイヤーオブジェクト:', newLayer);
      
      // Rustエンジンでテクスチャ作成
      console.log(`🦀 Rustエンジンでテクスチャ作成開始: ${newLayerId} (${project.width}x${project.height})`);
      await drawingEngine!.createLayer(newLayerId, project.width, project.height);
      console.log('✅ Rustエンジンテクスチャ作成完了');
      
      // プロジェクト状態を更新
      const updatedProject = { ...project };
      const frameIndex = updatedProject.frames.findIndex((f: any) => f.id === currentFrame.id);
      
      console.log(`📝 フレーム更新: frameIndex=${frameIndex}, frameId=${currentFrame.id}`);
      
      if (frameIndex !== -1) {
        const oldLayerCount = updatedProject.frames[frameIndex].layers.length;
        updatedProject.frames[frameIndex].layers = [...layers, newLayer];
        const newLayerCount = updatedProject.frames[frameIndex].layers.length;
        
        console.log(`📈 レイヤー数変更: ${oldLayerCount} → ${newLayerCount}`);
        
        setProject(updatedProject);
        setSelectedLayer(newLayerId);
        
        console.log(`✅ レイヤー作成完了: ${newLayerId}`);
        setDebugInfo(prev => ({ 
          ...prev, 
          layerCount: newLayerCount,
          engineState: 'レイヤー作成成功',
          lastError: null
        }));
      } else {
        throw new Error(`フレームが見つかりません (frameId: ${currentFrame.id})`);
      }
      
      setError(null);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      console.error('❌ レイヤー作成エラー詳細:', {
        error: errorMsg,
        stack: error instanceof Error ? error.stack : undefined,
        projectId: project?.id,
        frameId: currentFrame?.id,
        engineInitialized: isEngineInitialized,
        timestamp: new Date().toISOString()
      });
      
      setError(`レイヤー作成失敗: ${errorMsg}`);
      setDebugInfo(prev => ({ 
        ...prev, 
        engineState: 'レイヤー作成エラー',
        lastError: errorMsg
      }));
    } finally {
      setIsCreating(false);
    }
  }, [project, currentFrame, layers, canAddLayer, setProject, setSelectedLayer, isEngineInitialized, drawingEngine]);

  // レイヤー削除
  const handleDeleteLayer = useCallback(async (layerId: string) => {
    if (!project || !currentFrame) return;
    
    try {
      // Rustエンジンからテクスチャを削除
      await drawingEngine!.removeLayer(layerId);
      
      // プロジェクト状態を更新
      const updatedProject = { ...project };
      const frameIndex = updatedProject.frames.findIndex((f: any) => f.id === currentFrame.id);
      if (frameIndex !== -1) {
        updatedProject.frames[frameIndex].layers = layers.filter((l: any) => l.id !== layerId);
        setProject(updatedProject);
        
        // 削除されたレイヤーが選択中だった場合、選択を解除
        if (selectedLayer === layerId) {
          const remainingLayers = updatedProject.frames[frameIndex].layers;
          setSelectedLayer(remainingLayers.length > 0 ? remainingLayers[0].id : null);
        }
      }
      
      setShowDeleteConfirm(null);
      setError(null);
    } catch (error) {
      console.error('レイヤー削除エラー:', error);
      setError('レイヤーの削除に失敗しました');
    }
  }, [project, currentFrame, layers, selectedLayer, setProject, setSelectedLayer]);

  // レイヤー名編集
  const handleEditLayerName = useCallback((layerId: string, currentName: string) => {
    setEditingLayerId(layerId);
    setEditingName(currentName);
  }, []);

  const handleSaveLayerName = useCallback(async () => {
    if (!project || !currentFrame || !editingLayerId) return;
    
    const updatedProject = { ...project };
    const frameIndex = updatedProject.frames.findIndex((f: any) => f.id === currentFrame.id);
    if (frameIndex !== -1) {
      const layerIndex = updatedProject.frames[frameIndex].layers.findIndex((l: any) => l.id === editingLayerId);
      if (layerIndex !== -1) {
        updatedProject.frames[frameIndex].layers[layerIndex].name = editingName.trim() || `レイヤー ${layerIndex + 1}`;
        setProject(updatedProject);
      }
    }
    
    setEditingLayerId(null);
    setEditingName('');
  }, [project, currentFrame, editingLayerId, editingName, setProject]);

  // レイヤー表示/非表示切り替え
  const toggleLayerVisibility = useCallback((layerId: string) => {
    if (!project || !currentFrame) return;
    
    const updatedProject = { ...project };
    const frameIndex = updatedProject.frames.findIndex((f: any) => f.id === currentFrame.id);
    if (frameIndex !== -1) {
      const layerIndex = updatedProject.frames[frameIndex].layers.findIndex((l: any) => l.id === layerId);
      if (layerIndex !== -1) {
        updatedProject.frames[frameIndex].layers[layerIndex].visible = !updatedProject.frames[frameIndex].layers[layerIndex].visible;
        setProject(updatedProject);
      }
    }
  }, [project, currentFrame, setProject]);

  // レイヤーロック切り替え
  const toggleLayerLock = useCallback((layerId: string) => {
    if (!project || !currentFrame) return;
    
    const updatedProject = { ...project };
    const frameIndex = updatedProject.frames.findIndex((f: any) => f.id === currentFrame.id);
    if (frameIndex !== -1) {
      const layerIndex = updatedProject.frames[frameIndex].layers.findIndex((l: any) => l.id === layerId);
      if (layerIndex !== -1) {
        updatedProject.frames[frameIndex].layers[layerIndex].locked = !updatedProject.frames[frameIndex].layers[layerIndex].locked;
        setProject(updatedProject);
      }
    }
  }, [project, currentFrame, setProject]);

  return (
    <div className="fixed top-4 right-4 w-72 max-h-96 bg-secondary-800 rounded-xl shadow-2xl border border-secondary-600 overflow-hidden z-50">
      {/* ヘッダー */}
      <div className="bg-secondary-700 px-4 py-3 border-b border-secondary-600">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-medium text-secondary-100">レイヤー</h3>
            <span className="text-xs text-secondary-400">({layers.length}/{maxLayers})</span>
          </div>
          <div className="flex items-center gap-1">
            <Button 
              variant="icon" 
              icon={isCreating ? (
                <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v4m0 4v4m4-8h-4m4 0h-4" />
                </svg>
              ) : <PlusIcon />} 
              className={`h-6 w-6 p-1 ${canAddLayer && !isCreating ? 'hover:bg-primary-600' : 'opacity-50 cursor-not-allowed'}`}
              onClick={canAddLayer && !isCreating ? handleCreateLayer : undefined}
              disabled={!canAddLayer || !isEngineInitialized || isCreating}
              title={
                isCreating ? 'レイヤー作成中...' :
                !isEngineInitialized ? '描画エンジン初期化中...' :
                canAddLayer ? '新しいレイヤーを作成' : `最大${maxLayers}枚まで`
              }
            />
            <Button 
              variant="icon" 
              icon={<TrashIcon />} 
              className={`h-6 w-6 p-1 ${selectedLayer ? 'hover:bg-red-600' : 'opacity-50 cursor-not-allowed'}`}
              onClick={selectedLayer ? () => setShowDeleteConfirm(selectedLayer) : undefined}
              disabled={!selectedLayer}
              title="選択中のレイヤーを削除"
            />
          </div>
        </div>
        
        {/* エラー表示 */}
        {displayError && (
          <div className="mt-2 space-y-2">
            <div className="p-2 bg-red-900/50 border border-red-600 rounded text-xs text-red-200">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="font-medium">エラー発生</div>
                  <div className="mt-1">{displayError}</div>
                  {debugInfo.lastError && debugInfo.lastError !== displayError && (
                    <div className="mt-1 text-red-300">詳細: {debugInfo.lastError}</div>
                  )}
                </div>
                <button 
                  onClick={() => {
                    setError(null);
                    clearError();
                    setDebugInfo(prev => ({ ...prev, lastError: null }));
                  }}
                  className="ml-2 text-red-400 hover:text-red-200 flex-shrink-0"
                >
                  ×
                </button>
              </div>
              {error && (
                <button 
                  onClick={handleCreateLayer}
                  disabled={isCreating || !isEngineInitialized}
                  className="mt-2 px-2 py-1 bg-red-700 hover:bg-red-600 disabled:opacity-50 rounded text-xs"
                >
                  {isCreating ? '再試行中...' : '再試行'}
                </button>
              )}
            </div>
          </div>
        )}
        
        {/* 状態表示 */}
        {!displayError && (
          <div className="mt-2 space-y-1">
            {!isEngineInitialized && (
              <div className="p-2 bg-yellow-900/50 border border-yellow-600 rounded text-xs text-yellow-200">
                <div className="flex items-center gap-2">
                  <svg className="w-3 h-3 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v4m0 4v4m4-8h-4m4 0h-4" />
                  </svg>
                  描画エンジンを初期化中...
                </div>
              </div>
            )}
            
            {isCreating && (
              <div className="p-2 bg-blue-900/50 border border-blue-600 rounded text-xs text-blue-200">
                <div className="flex items-center gap-2">
                  <svg className="w-3 h-3 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v4m0 4v4m4-8h-4m4 0h-4" />
                  </svg>
                  レイヤーを作成中...
                </div>
              </div>
            )}
            
            {debugInfo.engineState && (
              <div className="p-2 bg-secondary-700 border border-secondary-600 rounded text-xs text-secondary-300">
                状態: {debugInfo.engineState}
              </div>
            )}
          </div>
        )}
        
        {/* デバッグ情報表示（開発時のみ） */}
        {typeof window !== 'undefined' && debugInfo.projectState && (
          <details className="mt-2">
            <summary className="text-xs text-secondary-400 cursor-pointer hover:text-secondary-300">
              デバッグ情報
            </summary>
            <div className="mt-1 p-2 bg-secondary-900 border border-secondary-700 rounded text-xs text-secondary-300 font-mono">
              <div>プロジェクト: {debugInfo.projectState.hasProject ? '✓' : '✗'}</div>
              <div>フレーム: {debugInfo.projectState.hasCurrentFrame ? '✓' : '✗'}</div>
              <div>エンジン: {debugInfo.projectState.isEngineInitialized ? '✓' : '✗'}</div>
              <div>レイヤー数: {debugInfo.layerCount}/{maxLayers}</div>
              {debugInfo.projectState.projectDimensions && (
                <div>サイズ: {debugInfo.projectState.projectDimensions}</div>
              )}
            </div>
          </details>
        )}
      </div>

      {/* レイヤーリスト */}
      <div className="overflow-y-auto max-h-80 p-2">
        <div className="space-y-1">
          {layers.map((layer: any) => (
            <div
              key={layer.id}
              className={`layer-item group rounded-lg p-2 cursor-pointer transition-colors ${
                selectedLayer === layer.id 
                  ? 'bg-primary-600 hover:bg-primary-500 ring-2 ring-primary-400' 
                  : 'hover:bg-secondary-700'
              }`}
              onClick={() => setSelectedLayer(layer.id)}
            >
              <div className="flex items-center gap-2">
                <Button
                  variant="icon"
                  icon={<EyeIcon visible={layer.visible} />}
                  className={`p-1 h-6 w-6 ${layer.visible ? 'text-primary-400' : 'text-secondary-500'}`}
                  onClick={(e?: React.MouseEvent<HTMLButtonElement>) => {
                    e?.stopPropagation();
                    toggleLayerVisibility(layer.id);
                  }}
                  title={layer.visible ? 'レイヤーを非表示' : 'レイヤーを表示'}
                />
                <Button
                  variant="icon"
                  icon={<LockIcon locked={layer.locked} />}
                  className={`p-1 h-6 w-6 ${layer.locked ? 'text-red-400' : 'text-secondary-500'}`}
                  onClick={(e?: React.MouseEvent<HTMLButtonElement>) => {
                    e?.stopPropagation();
                    toggleLayerLock(layer.id);
                  }}
                  title={layer.locked ? 'レイヤーのロックを解除' : 'レイヤーをロック'}
                />
                
                <div className="flex-1 min-w-0">
                  {editingLayerId === layer.id ? (
                    <input
                      type="text"
                      value={editingName}
                      onChange={(e) => setEditingName(e.target.value)}
                      onBlur={handleSaveLayerName}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleSaveLayerName();
                        if (e.key === 'Escape') {
                          setEditingLayerId(null);
                          setEditingName('');
                        }
                      }}
                      className="w-full px-1 py-0.5 text-sm bg-secondary-800 border border-secondary-600 rounded text-secondary-100 focus:outline-none focus:border-primary-500"
                      autoFocus
                      onClick={(e: React.MouseEvent<HTMLInputElement>) => e.stopPropagation()}
                    />
                  ) : (
                    <div
                      className="text-sm font-medium text-secondary-100 truncate cursor-text"
                      onDoubleClick={(e: React.MouseEvent<HTMLDivElement>) => {
                        e.stopPropagation();
                        handleEditLayerName(layer.id, layer.name);
                      }}
                      title="ダブルクリックで名前を編集"
                    >
                      {layer.name}
                    </div>
                  )}
                  <div className="text-xs text-secondary-400">
                    {layer.blendMode} • {Math.round(layer.opacity * 100)}%
                  </div>
                </div>
                
                <Button
                  variant="icon"
                  icon={<EditIcon />}
                  className="p-1 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={(e?: React.MouseEvent<HTMLButtonElement>) => {
                    e?.stopPropagation();
                    handleEditLayerName(layer.id, layer.name);
                  }}
                  title="レイヤー名を編集"
                />
                
                <LayerThumbnail layerId={layer.id} drawingEngine={drawingEngine} />
              </div>
            </div>
          ))}
          
          {layers.length === 0 && (
            <div className="text-center text-secondary-400 py-8">
              <p>レイヤーがありません</p>
              <p className="text-xs mt-1">「+」ボタンでレイヤーを作成</p>
            </div>
          )}
        </div>
      </div>
      
      {/* 削除確認ダイアログ */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-secondary-800 rounded-lg border border-secondary-600 p-4 max-w-sm mx-4">
            <h4 className="text-lg font-medium text-secondary-100 mb-2">レイヤーを削除</h4>
            <p className="text-sm text-secondary-300 mb-4">
              「{layers.find((l: any) => l.id === showDeleteConfirm)?.name}」を削除しますか？
              <br />
              <span className="text-red-400">この操作は取り消せません。</span>
            </p>
            <div className="flex gap-2 justify-end">
              <Button
                variant="secondary"
                onClick={() => setShowDeleteConfirm(null)}
                className="px-3 py-1 text-sm"
              >
                キャンセル
              </Button>
              <Button
                variant="danger"
                onClick={() => handleDeleteLayer(showDeleteConfirm)}
                className="px-3 py-1 text-sm bg-red-600 hover:bg-red-700"
              >
                削除
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}