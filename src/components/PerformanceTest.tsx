import { useState, useCallback } from 'react';
import { useDrawingEngine } from '../lib/useDrawingEngine';
import { DrawEngineCommand } from '../lib/bindings';
import { PerformanceMeasure } from '../lib/binaryProtocol';

interface TestResult {
  method: 'JSON' | 'Binary';
  commandCount: number;
  dataSize: number; // bytes
  sendTime: number; // ms
  throughput: number; // commands/ms
  avgCommandTime: number; // ms per command
}

export function PerformanceTest() {
  const [isRunning, setIsRunning] = useState(false);
  const [results, setResults] = useState<TestResult[]>([]);
  const [pointCount, setPointCount] = useState(1000);
  
  // JSONモードの描画エンジン
  const jsonEngine = useDrawingEngine({ useBinary: false });
  const binaryEngine = useDrawingEngine({ useBinary: true });
  
  // テスト用のランダムポイント生成
  const generateRandomPoints = useCallback((count: number) => {
    const points: Array<{ x: number; y: number; pressure: number }> = [];
    let x = Math.random() * 800;
    let y = Math.random() * 600;
    
    for (let i = 0; i < count; i++) {
      // ランダムウォークで連続的な線を生成
      x += (Math.random() - 0.5) * 20;
      y += (Math.random() - 0.5) * 20;
      
      // キャンバス範囲内に収める
      x = Math.max(0, Math.min(800, x));
      y = Math.max(0, Math.min(600, y));
      
      points.push({
        x,
        y,
        pressure: 0.5 + Math.random() * 0.5,
      });
    }
    
    return points;
  }, []);
  
  // データサイズ推定
  const estimateDataSize = useCallback((commands: DrawEngineCommand[]): number => {
    let size = 0;
    
    for (const cmd of commands) {
      if (typeof cmd === 'object' && ('beginStroke' in cmd || 'continueStroke' in cmd)) {
        // JSON: 約60-80バイト per stroke command
        // Binary: 13バイト (1 type + 3 * 4 float)
        size += 70; // JSON平均推定値
      } else if (cmd === 'endStroke') {
        // JSON: 約15バイト
        // Binary: 1バイト
        size += 15; // JSON推定値
      }
    }
    
    return size;
  }, []);
  
  // パフォーマンステスト実行
  const runPerformanceTest = useCallback(async () => {
    setIsRunning(true);
    setResults([]);
    
    try {
      // キャンバス初期化
      await jsonEngine.initEngine(800, 600);
      await binaryEngine.initEngine(800, 600);
      
      // テストポイント生成
      const points = generateRandomPoints(pointCount);
      
      // コマンド配列を生成
      const commands: DrawEngineCommand[] = [];
      
      // 複数のストロークを生成
      const strokeCount = Math.floor(pointCount / 100); // 100ポイントごとに1ストローク
      let pointIndex = 0;
      
      for (let s = 0; s < strokeCount; s++) {
        const strokePoints = Math.min(100, pointCount - pointIndex);
        
        if (strokePoints > 0) {
          // beginStroke
          const firstPoint = points[pointIndex];
          commands.push({ beginStroke: firstPoint });
          pointIndex++;
          
          // continueStroke
          for (let i = 1; i < strokePoints && pointIndex < points.length; i++) {
            commands.push({ continueStroke: points[pointIndex] });
            pointIndex++;
          }
          
          // endStroke
          commands.push('endStroke');
        }
      }
      
      console.log(`Generated ${commands.length} commands for ${pointCount} points`);
      
      // JSONモードのテスト
      const jsonPerf = new PerformanceMeasure();
      jsonPerf.mark('start');
      
      // バッチ処理を無効化して直接送信
      jsonEngine.setUseBatching(false);
      
      for (const cmd of commands) {
        await jsonEngine.sendCommand(cmd);
      }
      
      jsonPerf.mark('end');
      const jsonReport = jsonPerf.getReport();
      
      const jsonResult: TestResult = {
        method: 'JSON',
        commandCount: commands.length,
        dataSize: estimateDataSize(commands),
        sendTime: jsonReport.total,
        throughput: commands.length / jsonReport.total,
        avgCommandTime: jsonReport.total / commands.length,
      };
      
      setResults(prev => [...prev, jsonResult]);
      
      // 少し待機
      await new Promise(resolve => setTimeout(resolve, 500));
      
      // バイナリモードのテスト
      const binaryPerf = new PerformanceMeasure();
      binaryPerf.mark('start');
      
      // バッチ処理を無効化して直接送信
      binaryEngine.setUseBatching(false);
      
      for (const cmd of commands) {
        await binaryEngine.sendCommand(cmd);
      }
      
      binaryPerf.mark('end');
      const binaryReport = binaryPerf.getReport();
      
      const binaryResult: TestResult = {
        method: 'Binary',
        commandCount: commands.length,
        dataSize: commands.length * 13, // 平均的なバイナリサイズ
        sendTime: binaryReport.total,
        throughput: commands.length / binaryReport.total,
        avgCommandTime: binaryReport.total / commands.length,
      };
      
      setResults(prev => [...prev, binaryResult]);
      
      // バッチ処理テスト（バイナリモード）
      await new Promise(resolve => setTimeout(resolve, 500));
      
      const batchPerf = new PerformanceMeasure();
      batchPerf.mark('start');
      
      // バッチ処理を有効化
      binaryEngine.setUseBatching(true);
      
      for (const cmd of commands) {
        await binaryEngine.sendCommand(cmd);
      }
      
      // 最後のバッチをフラッシュ
      await new Promise(resolve => setTimeout(resolve, 100));
      
      batchPerf.mark('end');
      const batchReport = batchPerf.getReport();
      
      const batchResult: TestResult = {
        method: 'Binary',
        commandCount: commands.length,
        dataSize: commands.length * 13,
        sendTime: batchReport.total,
        throughput: commands.length / batchReport.total,
        avgCommandTime: batchReport.total / commands.length,
      };
      
      setResults(prev => [...prev, { ...batchResult, method: 'Binary' as const }]);
      
    } catch (error) {
      console.error('Performance test failed:', error);
    } finally {
      setIsRunning(false);
    }
  }, [pointCount, jsonEngine, binaryEngine, generateRandomPoints, estimateDataSize]);
  
  // 結果の比較計算
  const calculateImprovement = useCallback(() => {
    if (results.length < 2) return null;
    
    const jsonResult = results.find(r => r.method === 'JSON');
    const binaryResult = results.find(r => r.method === 'Binary');
    
    if (!jsonResult || !binaryResult) return null;
    
    return {
      timeImprovement: ((jsonResult.sendTime - binaryResult.sendTime) / jsonResult.sendTime * 100).toFixed(1),
      dataSizeReduction: ((jsonResult.dataSize - binaryResult.dataSize) / jsonResult.dataSize * 100).toFixed(1),
      throughputIncrease: ((binaryResult.throughput - jsonResult.throughput) / jsonResult.throughput * 100).toFixed(1),
    };
  }, [results]);
  
  const improvement = calculateImprovement();
  
  return (
    <div className="p-6 bg-white rounded-lg shadow-lg max-w-4xl mx-auto">
      <h2 className="text-2xl font-bold mb-6">描画エンジン パフォーマンステスト</h2>
      
      <div className="mb-6">
        <label className="block text-sm font-medium mb-2">
          テストポイント数: {pointCount}
        </label>
        <input
          type="range"
          min="100"
          max="10000"
          step="100"
          value={pointCount}
          onChange={(e) => setPointCount(Number(e.target.value))}
          className="w-full"
          disabled={isRunning}
        />
        <div className="flex justify-between text-sm text-gray-600">
          <span>100</span>
          <span>5,000</span>
          <span>10,000</span>
        </div>
      </div>
      
      <button
        onClick={runPerformanceTest}
        disabled={isRunning}
        className={`px-6 py-3 rounded-lg font-medium transition-colors ${
          isRunning
            ? 'bg-gray-300 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700 text-white'
        }`}
      >
        {isRunning ? 'テスト実行中...' : 'パフォーマンステスト開始'}
      </button>
      
      {results.length > 0 && (
        <div className="mt-8">
          <h3 className="text-lg font-semibold mb-4">テスト結果</h3>
          
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    送信方式
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    コマンド数
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    データサイズ
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    送信時間
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    スループット
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    平均コマンド時間
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {results.map((result, index) => (
                  <tr key={index}>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                      {result.method}
                      {index === 2 && ' (バッチ)'}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {result.commandCount.toLocaleString()}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {(result.dataSize / 1024).toFixed(2)} KB
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {result.sendTime.toFixed(2)} ms
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {result.throughput.toFixed(2)} cmd/ms
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {result.avgCommandTime.toFixed(3)} ms
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          
          {improvement && (
            <div className="mt-6 p-4 bg-green-50 rounded-lg">
              <h4 className="font-semibold text-green-800 mb-2">バイナリ送信の改善効果</h4>
              <ul className="space-y-1 text-green-700">
                <li>• 送信時間: {improvement.timeImprovement}% 短縮</li>
                <li>• データサイズ: {improvement.dataSizeReduction}% 削減</li>
                <li>• スループット: {improvement.throughputIncrease}% 向上</li>
              </ul>
            </div>
          )}
        </div>
      )}
      
      <div className="mt-8 text-sm text-gray-600">
        <p className="font-semibold mb-2">テスト内容:</p>
        <ul className="space-y-1">
          <li>• 指定数のランダムポイントを生成し、描画コマンドとして送信</li>
          <li>• JSON形式とバイナリ形式の送信時間を比較</li>
          <li>• バッチ処理の効果も測定</li>
        </ul>
      </div>
    </div>
  );
}