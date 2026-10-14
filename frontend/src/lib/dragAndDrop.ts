import { useRef, useState, type DragEvent } from "react";

// Two independent drag systems:
//  - useDragReorder: reorder items within one list (stage rows, category rows).
//  - map dragging: a map card carries its id under MAP_DRAG_TYPE and slots / the
//    generic pool accept it via useMapDropZone.

export function useDragReorder(onMove: (draggedId: string, targetId: string) => void) {
  const dragged = useRef<string | null>(null);

  return (id: string) => ({
    draggable: true,
    onDragStart: () => {
      dragged.current = id;
    },
    onDragOver: (e: DragEvent) => {
      if (dragged.current) e.preventDefault();
    },
    onDrop: (e: DragEvent) => {
      if (!dragged.current) return;
      e.preventDefault();
      onMove(dragged.current, id);
      dragged.current = null;
    },
    onDragEnd: () => {
      dragged.current = null;
    },
  });
}

export const MAP_DRAG_TYPE = "application/x-pool-map-id";

export function mapDragProps(mapId: string) {
  return {
    draggable: true,
    onDragStart: (e: DragEvent) => {
      e.dataTransfer.setData(MAP_DRAG_TYPE, mapId);
      e.dataTransfer.effectAllowed = "move";
    },
  };
}

export function useMapDropZone(onDropMap: (mapId: string) => void) {
  const [isOver, setIsOver] = useState(false);
  // dragenter/dragleave also fire for child elements, so count depth to know when
  // the pointer really left the zone.
  const depth = useRef(0);

  const carriesMap = (e: DragEvent) => e.dataTransfer.types.includes(MAP_DRAG_TYPE);

  const dropProps = {
    onDragEnter: (e: DragEvent) => {
      if (!carriesMap(e)) return;
      depth.current += 1;
      setIsOver(true);
    },
    onDragLeave: (e: DragEvent) => {
      if (!carriesMap(e)) return;
      depth.current = Math.max(0, depth.current - 1);
      if (depth.current === 0) setIsOver(false);
    },
    onDragOver: (e: DragEvent) => {
      if (carriesMap(e)) e.preventDefault();
    },
    onDrop: (e: DragEvent) => {
      depth.current = 0;
      setIsOver(false);
      const mapId = e.dataTransfer.getData(MAP_DRAG_TYPE);
      if (!mapId) return;
      e.preventDefault();
      onDropMap(mapId);
    },
  };

  return { isOver, dropProps };
}
