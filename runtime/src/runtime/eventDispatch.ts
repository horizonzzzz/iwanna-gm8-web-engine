import type { RuntimeEventDispatchContext, RuntimeInstance } from './types';
import { interpretBlockSource } from './logicRunner';

export function dispatchEventBlocks(
  context: RuntimeEventDispatchContext,
  instance: RuntimeInstance,
  eventTag: string
): void {
  for (const block of instance.eventBlocks) {
    const prepared = context.pkg.blockMap.get(block.blockId);
    if (!prepared) {
      context.diagnostics.push({
        timestamp: context.tick,
        level: 'warning',
        code: 'runtime-missing-block',
        message: `Missing runtime block ${block.blockId}`,
        blockId: block.blockId,
        roomId: context.room.roomId,
        instanceId: instance.instanceId
      });
      continue;
    }

    if (!prepared.id || !prepared.kind || prepared.kind !== 'object-event' || block.eventTag !== eventTag) {
      continue;
    }

    interpretBlockSource(context, instance, prepared);
  }
}
