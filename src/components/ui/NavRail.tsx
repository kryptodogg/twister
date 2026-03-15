interface Page {
  id: string;
  label: string;
  icon: string;
}

interface NavRailProps {
  pages: Page[];
  active: string;
  onNavigate: (id: string) => void;
}

export function NavRail({ pages, active, onNavigate }: NavRailProps) {
  return (
    <nav className="h-full w-[72px] flex flex-col items-center py-4 bg-surface-header/50 border-r border-white/5 pt-14 no-drag">
      {pages.map((page) => {
        const isSelected = active === page.id;
        return (
          <button
            key={page.id}
            onClick={() => onNavigate(page.id)}
            className={`group relative flex flex-col items-center justify-center w-14 h-14 mb-2 rounded-2xl transition-all ${
              isSelected ? 'bg-br-teal/20 text-br-teal' : 'text-on-surface/50 hover:bg-white/5 hover:text-on-surface'
            }`}
            title={page.label}
          >
            {isSelected && (
              <div className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-br-teal rounded-r-full" />
            )}
            <span className="material-symbols-outlined text-2xl">{page.icon}</span>
            <span className="text-[10px] font-bold mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
               {page.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
