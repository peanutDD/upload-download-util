/**
 * DropdownMenu 组件
 *
 * 可复用的下拉菜单组件，支持自定义选项和回调。
 */
import React, { useState, useRef, useEffect } from "react";
import { ChevronDown } from "lucide-react";

interface DropdownMenuOption {
  label: string;
  value: string;
  icon?: React.ReactNode;
  /** 是否在此选项前显示分隔线 */
  divider?: boolean;
}

interface DropdownMenuProps {
  title: string;
  options: DropdownMenuOption[];
  selectedValue: string;
  onSelect: (value: string) => void;
  ariaLabel: string;
}

const DropdownMenu: React.FC<DropdownMenuProps> = ({
  title,
  options,
  selectedValue,
  onSelect,
  ariaLabel,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const triggerRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find(
    (option) => option.value === selectedValue,
  );
  const selectedLabel = selectedOption?.label || "";

  // 监听点击外部关闭下拉菜单
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      const isClickInsideTrigger =
        triggerRef.current?.contains(target) || false;
      const isClickInsideMenu = menuRef.current?.contains(target) || false;

      if (!isClickInsideTrigger && !isClickInsideMenu) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleToggle = () => {
    setIsOpen((prev) => !prev);
  };

  const handleOptionSelect = (value: string) => {
    onSelect(value);
    setIsOpen(false);
  };

  return (
    <div
      ref={triggerRef}
      className={isOpen ? "filtersCard filtersCardOpen" : "filtersCard"}
      data-oid="l.pv-2x"
    >
      <button
        type="button"
        className="filtersCardHeader"
        onClick={handleToggle}
        aria-label={ariaLabel}
        data-oid="18dcnzd"
      >
        <span className="filtersCardTitle" data-oid="5e7.k7g">
          {title}
        </span>
        <span className="filtersCardHeaderRight" data-oid="u8hoqwp">
          <span
            className="filtersCardSelected"
            title={selectedLabel}
            data-oid="8olbdny"
          >
            {selectedLabel}
          </span>
          <ChevronDown
            className={
              isOpen ? "filtersChevron filtersChevronOpen" : "filtersChevron"
            }
            aria-hidden="true"
            data-oid="ezmi5o6"
          />
        </span>
      </button>

      {isOpen && (
        <div
          ref={menuRef}
          className="filtersDropdownPanel"
          role="menu"
          aria-label={`${title} options`}
          data-oid="r2hbklo"
        >
          <div className="filtersList" data-oid="kkdb-13">
            {options.map((option) => (
              <React.Fragment key={option.value}>
                {option.divider && (
                  <div className="filtersDivider" data-oid="5mlbjfh" />
                )}
                <button
                  type="button"
                  role="menuitem"
                  className={
                    option.value === selectedValue
                      ? "filtersItem filtersItemSelected"
                      : "filtersItem"
                  }
                  onClick={() => handleOptionSelect(option.value)}
                  data-oid="thto2sa"
                >
                  {option.icon && (
                    <span className="filtersItemIcon" data-oid="eg_bcce">
                      {option.icon}
                    </span>
                  )}
                  <span className="filtersItemLabel" data-oid="nrd3qa6">
                    {option.label}
                  </span>
                  {option.value === selectedValue && (
                    <span className="filtersItemCheck" data-oid="shp3k64">
                      <i
                        className="bi bi-check2"
                        aria-hidden
                        data-oid="z1ee0z0"
                      />
                    </span>
                  )}
                </button>
              </React.Fragment>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default DropdownMenu;
