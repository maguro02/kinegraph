import React, { useState, useCallback } from 'react';
import { EyeIcon, EyeSlashIcon, TrashIcon, PlusIcon } from '@heroicons/react/24/outline';
import { useDrawingEngine } from '../lib/useDrawingEngine';
import { useAtom } from 'jotai';
import { drawingEngineStateAtom } from '../store/atoms';

interface Layer {
  id: number;
  name: string;
  visible: boolean;
  opacity: number;
}

export const LayerPanelDrawingEngine: React.FC = () => {
  const { createLayer, deleteLayer, setActiveLayer } = useDrawingEngine();
  const [engineState] = useAtom(drawingEngineStateAtom);
  const [layers, setLayers] = useState<Layer[]>([
    { id: 0, name: 'Layer 1', visible: true, opacity: 100 },
  ]);
  const [activeLayerIndex, setActiveLayerIndex] = useState(0);

  // レイヤーを追加
  const handleAddLayer = useCallback(async () => {
    // エンジンが初期化されていない場合は何もしない
    if (!engineState.isInitialized) {
      console.warn('Drawing engine is not initialized yet');
      return;
    }
    
    try {
      await createLayer();
      const newLayer: Layer = {
        id: layers.length,
        name: `Layer ${layers.length + 1}`,
        visible: true,
        opacity: 100,
      };
      setLayers([...layers, newLayer]);
      setActiveLayerIndex(layers.length);
      await setActiveLayer(layers.length);
    } catch (error) {
      console.error('Failed to create layer:', error);
    }
  }, [layers, createLayer, setActiveLayer, engineState.isInitialized]);

  // レイヤーを削除
  const handleDeleteLayer = useCallback(async (index: number) => {
    if (layers.length <= 1) {
      alert('最後のレイヤーは削除できません');
      return;
    }

    try {
      await deleteLayer(index);
      const newLayers = layers.filter((_, i) => i !== index);
      setLayers(newLayers);
      
      // アクティブレイヤーの調整
      if (activeLayerIndex >= newLayers.length) {
        const newActiveIndex = newLayers.length - 1;
        setActiveLayerIndex(newActiveIndex);
        await setActiveLayer(newActiveIndex);
      }
    } catch (error) {
      console.error('Failed to delete layer:', error);
    }
  }, [layers, activeLayerIndex, deleteLayer, setActiveLayer]);

  // レイヤーの可視性を切り替え
  const toggleLayerVisibility = useCallback((index: number) => {
    const newLayers = [...layers];
    newLayers[index].visible = !newLayers[index].visible;
    setLayers(newLayers);
    // TODO: 描画エンジンに可視性の変更を通知
  }, [layers]);

  // アクティブレイヤーを設定
  const handleSetActiveLayer = useCallback(async (index: number) => {
    setActiveLayerIndex(index);
    await setActiveLayer(index);
  }, [setActiveLayer]);

  // 不透明度を変更
  const handleOpacityChange = useCallback((index: number, opacity: number) => {
    const newLayers = [...layers];
    newLayers[index].opacity = opacity;
    setLayers(newLayers);
    // TODO: 描画エンジンに不透明度の変更を通知
  }, [layers]);

  return (
    <div className="bg-secondary-900 p-4 rounded-lg">
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-white font-semibold">レイヤー</h3>
        <button
          onClick={handleAddLayer}
          className="p-1 hover:bg-secondary-700 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          title={engineState.isInitialized ? "新しいレイヤーを追加" : "描画エンジンを初期化中..."}
          disabled={!engineState.isInitialized}
        >
          <PlusIcon className="w-5 h-5 text-white" />
        </button>
      </div>
      
      {/* エンジン未初期化の場合の表示 */}
      {!engineState.isInitialized && (
        <div className="text-center py-4 text-gray-400">
          <div className="mb-2">描画エンジンを初期化中...</div>
          <div className="text-xs">しばらくお待ちください</div>
        </div>
      )}
      
      <div className="space-y-2">
        {layers.slice().reverse().map((layer, reverseIndex) => {
          const index = layers.length - 1 - reverseIndex;
          const isActive = index === activeLayerIndex;
          
          return (
            <div
              key={layer.id}
              className={`p-3 rounded cursor-pointer transition-colors ${
                isActive
                  ? 'bg-primary-600 border-2 border-primary-400'
                  : 'bg-secondary-800 hover:bg-secondary-700 border-2 border-transparent'
              }`}
              onClick={() => handleSetActiveLayer(index)}
            >
              <div className="flex items-center justify-between mb-2">
                <span className="text-white font-medium">{layer.name}</span>
                <div className="flex gap-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleLayerVisibility(index);
                    }}
                    className="p-1 hover:bg-secondary-600 rounded transition-colors"
                  >
                    {layer.visible ? (
                      <EyeIcon className="w-4 h-4 text-white" />
                    ) : (
                      <EyeSlashIcon className="w-4 h-4 text-gray-500" />
                    )}
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteLayer(index);
                    }}
                    className="p-1 hover:bg-red-600 rounded transition-colors"
                    disabled={layers.length === 1}
                  >
                    <TrashIcon className={`w-4 h-4 ${
                      layers.length === 1 ? 'text-gray-600' : 'text-white'
                    }`} />
                  </button>
                </div>
              </div>
              
              <div className="flex items-center gap-2">
                <span className="text-xs text-gray-400">不透明度:</span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={layer.opacity}
                  onChange={(e) => {
                    e.stopPropagation();
                    handleOpacityChange(index, parseInt(e.target.value));
                  }}
                  className="flex-1 h-1"
                  onClick={(e) => e.stopPropagation()}
                />
                <span className="text-xs text-gray-400 w-8">{layer.opacity}%</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};