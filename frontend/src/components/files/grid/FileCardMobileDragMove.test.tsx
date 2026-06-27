import { act, fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FileMetadata } from "../../../types/files";
import FileCard from "./FileCard";

vi.mock("../preview/LazyThumbnail", () => ({
  default: () => <div data-testid="thumbnail" />,
}));

const file: FileMetadata = {
  id: "file-1",
  filename: "file-1.jpg",
  original_filename: "file-1.jpg",
  file_size: 1024,
  mime_type: "image/jpeg",
  category: "image",
  folder_id: null,
  created_at: "2026-05-07T08:00:00.000Z",
};

function fileCardProps(
  overrides: Partial<ComponentProps<typeof FileCard>> = {},
) {
  return {
    file,
    isSelected: false,
    onSelect: vi.fn(),
    onPreview: vi.fn(),
    onShare: vi.fn(),
    onDownload: vi.fn(),
    onRename: vi.fn(),
    onDelete: vi.fn(),
    isMenuOpen: false,
    onToggleMenu: vi.fn(),
    onCloseMenu: vi.fn(),
    ...overrides,
  };
}

function renderFile(overrides: Partial<ComponentProps<typeof FileCard>> = {}) {
  return render(<FileCard {...fileCardProps(overrides)} />);
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

describe("FileCard mobile drag move", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockPointerMedia(false);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("does not expose native HTML drag on coarse pointer devices", () => {
    renderFile();
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");

    expect(sourceCard).not.toHaveAttribute("draggable", "true");
  });

  it("keeps the unselected checkbox visible on coarse pointer devices", () => {
    renderFile();

    const checkbox = screen.getByRole("button", { name: "选择" });

    expect(checkbox).not.toHaveClass("invisible");
    expect(checkbox).toHaveClass("selection-checkbox-hover-reveal");
  });

  it("uses a high-contrast unselected checkbox treatment", () => {
    const { container } = renderFile();

    expect(container.querySelector(".card-checkbox-unselected")).toBeTruthy();
    expect(
      container.querySelector(".card-checkbox-unselected-ring"),
    ).toBeTruthy();
  });

  it("keeps native HTML drag on desktop pointer devices", () => {
    mockPointerMedia(true);
    renderFile();
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");

    expect(sourceCard).toHaveAttribute("draggable", "true");
  });

  it("prevents native long-press callout while a touch drag is pending", () => {
    renderFile();
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 7,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });

    expect(fireEvent.contextMenu(sourceCard as Element)).toBe(false);
  });

  it("starts mobile file drag only after long press and drops on a target folder", () => {
    const onMobileFileDrop = vi.fn();
    const onMobileFileDragStart = vi.fn();
    const onMobileFileDragEnd = vi.fn();
    const { container } = renderFile({
      onMobileFileDrop,
      onMobileFileDragStart,
      onMobileFileDragEnd,
    });
    const targetFolder = document.createElement("div");
    targetFolder.dataset.folderId = "folder-target";
    document.body.appendChild(targetFolder);

    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
    expect(sourceCard).toBeTruthy();
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => targetFolder),
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
    expect(onMobileFileDragStart).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(onMobileFileDragStart).toHaveBeenCalledWith(file.id);
    const draggingCard = container.querySelector("[data-mobile-file-dragging='true']");
    expect(draggingCard).toBeTruthy();
    expect(draggingCard).toHaveClass("pointer-events-none");

    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 1,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFileDrop).toHaveBeenCalledWith("folder-target", file.id);
    expect(onMobileFileDragEnd).toHaveBeenCalledTimes(1);
  });

  it("starts mobile file drag when long pressing the hidden preview action", () => {
    const onMobileFileDrop = vi.fn();
    const onMobileFileDragStart = vi.fn();
    renderFile({ onMobileFileDrop, onMobileFileDragStart });
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
    const previewButton = screen.getByRole("button", { name: "预览" });
    const targetFolder = document.createElement("div");
    targetFolder.dataset.folderId = "folder-target";

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => targetFolder),
    });

    fireEvent.pointerDown(previewButton, {
      pointerId: 5,
      pointerType: "touch",
      clientX: 32,
      clientY: 32,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 5,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFileDragStart).toHaveBeenCalledWith(file.id);
    expect(onMobileFileDrop).toHaveBeenCalledWith("folder-target", file.id);
  });

  it("finishes mobile file drag when pointerup is delivered outside the source card", () => {
    const onMobileFileDrop = vi.fn();
    renderFile({ onMobileFileDrop });
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
    const targetFolder = document.createElement("div");
    targetFolder.dataset.folderId = "folder-target";

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => targetFolder),
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 6,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    fireEvent.pointerUp(window, {
      pointerId: 6,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    expect(onMobileFileDrop).toHaveBeenCalledWith("folder-target", file.id);
  });

  it("resets suppressed preview state on the next pointer interaction", () => {
    const onPreview = vi.fn();
    renderFile({ onPreview });
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
    const thumbnail = screen.getByTestId("thumbnail").parentElement;
    const targetFolder = document.createElement("div");
    targetFolder.dataset.folderId = "folder-target";
    document.body.appendChild(targetFolder);

    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => targetFolder),
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 1,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    act(() => {
      vi.advanceTimersByTime(450);
    });
    fireEvent.pointerUp(sourceCard as Element, {
      pointerId: 1,
      pointerType: "touch",
      clientX: 120,
      clientY: 120,
      isPrimary: true,
    });

    fireEvent.pointerDown(sourceCard as Element, {
      pointerId: 2,
      pointerType: "touch",
      clientX: 16,
      clientY: 16,
      isPrimary: true,
    });
    fireEvent.click(thumbnail as Element);

    expect(onPreview).toHaveBeenCalledWith(file);
  });

  it("uses updated callback props after parent rerender", () => {
    const firstPreview = vi.fn();
    const secondPreview = vi.fn();
    const { rerender } = renderFile({ onPreview: firstPreview });
    const thumbnail = screen.getByTestId("thumbnail").parentElement;

    rerender(
      <FileCard
        {...fileCardProps({
          onPreview: secondPreview,
        })}
      />,
    );
    fireEvent.click(thumbnail as Element);

    expect(firstPreview).not.toHaveBeenCalled();
    expect(secondPreview).toHaveBeenCalledWith(file);
  });

  it("drops mobile files onto the root folder sentinel", () => {
    const onMobileFileDrop = vi.fn();
    renderFile({ onMobileFileDrop });
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
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

    expect(onMobileFileDrop).toHaveBeenCalledWith("", file.id);
  });

  it("finds a covered folder target from the pointer hit stack", () => {
    const onMobileFileDrop = vi.fn();
    renderFile({ onMobileFileDrop });
    const sourceCard = screen.getByTitle(file.original_filename).closest("[data-file-id]");
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

    expect(onMobileFileDrop).toHaveBeenCalledWith("folder-target", file.id);
  });
});
