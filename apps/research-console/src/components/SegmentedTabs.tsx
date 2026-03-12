export function SegmentedTabs({
  items,
  value,
  onChange,
}: {
  items: Array<{ value: string; label: string }>;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="segmented-tabs" role="tablist" aria-label="View switcher">
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          role="tab"
          aria-selected={value === item.value}
          className={`segmented-tab${value === item.value ? " segmented-tab-active" : ""}`}
          onClick={() => onChange(item.value)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
