import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Folder } from "../../../types/folders";
import FolderCard from "./FolderCard";

const sourceFolder: Folder = {
  id: "folder-source",
  name: "Source",
  parent_id: null,
  created_at: "2026-05-07T08:00:00.000Z",
  updated_at: "2026-05-07T08:00:00.000Z",
};

const targetFolder: Folder = {
  id: "folder-target",
  name: "Target",
  parent_id: null,
  created_at: "2026-05-07T08:00:00.000Z",
  updated_at: "2026-05-07T08:00:00.000Z",
};

function renderFolder(
  folder: Folder,
  onMobileFolderDrop = vi.fn(),
  onMobileFolderDragStart = vi.fn(),
  onMobileFolderDragEnd = vi.fn(),
  onDrop = vi.fn(),
) {
  return render(
    <FolderCard
      folder={folder}
      isSelected={false}
      onSelect={vi.fn()}
      onOpen={vi.fn()}
      onRename={vi.fn()}
      onDelete={vi.fn()}
      isMenuOpen={false}
      onToggleMenu={vi.fn()}
      onCloseMenu={vi.fn()}
      onMobileFolderDrop={onMobileFolderDrop}
      onMobileFolderDragStart={onMobileFolderDragStart}
      onMobileFolderDragEnd={onMobileFolderDragEnd}
      onDrop={onDrop}
    />,
  );
}

function mockPointerMedia(isDesktopPointer: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn((query: string) => ({
      matches:
        query === "(hover: hover) and (pointer: fine)" && isDesktopPointer,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

describe("FolderCard drag move", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockPointerMedia(false);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("does not expose native folder drag on coarse pointer devices", () => {
    renderFolder(sourceFolder);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");
    const draggableThumb = sourceCard?.querySelector("[draggable='true']");

    expect(draggableThumb).toBeNull();
  });

  it("keeps native folder drag on desktop pointer devices", () => {
    mockPointerMedia(true);
    renderFolder(sourceFolder);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");
    const draggableThumb = sourceCard?.querySelector("[draggable='true']");

    expect(draggableThumb).toBeTruthy();
  });

  it("prevents native long-press callout while a touch folder drag is pending", () => {
    renderFolder(sourceFolder);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 5,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });

    expect(fireEvent.contextMenu(sourceCard as Element)).toBe(false);
  });

  it("accepts desktop file drops on the full folder card", () => {
    const onDrop = vi.fn();
    renderFolder(targetFolder, vi.fn(), vi.fn(), vi.fn(), onDrop);
    const targetCard = screen.getByTitle(targetFolder.name).closest("[data-folder-id]");
    expect(targetCard).toBeTruthy();

    const dataTransfer = {
      types: ["application/file-id"],
      dropEffect: "none",
      getData: vi.fn((type: string) =>
        type === "application/file-id" ? "file-1" : "",
      ),
    };

    fireEvent.dragOver(targetCard as Element, { dataTransfer });
    fireEvent.drop(targetCard as Element, { dataTransfer });

    expect(dataTransfer.dropEffect).toBe("move");
    expect(onDrop).toHaveBeenCalledWith(targetFolder.id, ["file-1"], []);
  });

  it("starts mobile folder drag only after a long press and drops on the folder under release", () => {
    const onMobileFolderDrop = vi.fn();
    const onMobileFolderDragStart = vi.fn();
    const onMobileFolderDragEnd = vi.fn();

    const { container } = renderFolder(
      sourceFolder,
      onMobileFolderDrop,
      onMobileFolderDragStart,
      onMobileFolderDragEnd,
    );
    renderFolder(targetFolder, onMobileFolderDrop);

    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");
    const targetCard = screen.getByTitle(targetFolder.name).closest("[data-folder-id]");
    expect(sourceCard).toBeTruthy();
    expect(targetCard).toBeTruthy();

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => targetCard),
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 1,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });

    act(() => {
      vi.advanceTimersByTime(449);
    });
    expect(onMobileFolderDragStart).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(onMobileFolderDragStart).toHaveBeenCalledWith(sourceFolder.id);
    const draggingCard = container.querySelector("[data-mobile-folder-dragging='true']");
    expect(draggingCard).toBeTruthy();
    expect(draggingCard).toHaveClass("pointer-events-none");

    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 1,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFolderDrop).toHaveBeenCalledWith(targetFolder.id, sourceFolder.id);
    expect(onMobileFolderDragEnd).toHaveBeenCalledTimes(1);
  });

  it("does not start mobile folder drag for a short tap", () => {
    const onMobileFolderDrop = vi.fn();
    const onMobileFolderDragStart = vi.fn();

    renderFolder(sourceFolder, onMobileFolderDrop, onMobileFolderDragStart);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 2,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(120);
    });
    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 2,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });

    expect(onMobileFolderDragStart).not.toHaveBeenCalled();
    expect(onMobileFolderDrop).not.toHaveBeenCalled();
  });

  it("drops mobile folders onto the root folder sentinel", () => {
    const onMobileFolderDrop = vi.fn();
    renderFolder(sourceFolder, onMobileFolderDrop);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");
    const rootTarget = document.createElement("div");
    rootTarget.dataset.folderId = "";

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => rootTarget),
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 3,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 3,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFolderDrop).toHaveBeenCalledWith("", sourceFolder.id);
  });

  it("finds a covered folder target from the pointer hit stack", () => {
    const onMobileFolderDrop = vi.fn();
    renderFolder(sourceFolder, onMobileFolderDrop);
    const sourceCard = screen.getByTitle(sourceFolder.name).closest("[data-folder-id]");
    const targetFolder = document.createElement("div");
    targetFolder.dataset.folderId = "folder-target";

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => sourceCard),
    });
    Object.defineProperty(document, "elementsFromPoint", {
      configurable: true,
      value: vi.fn(() => [sourceCard, targetFolder]),
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 4,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 4,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFolderDrop).toHaveBeenCalledWith(
      "folder-target",
      sourceFolder.id,
    );
  });
});
