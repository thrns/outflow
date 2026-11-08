export type AppPage = "home" | "settings";
// Welcome is transient Home flow state, not a third primary navigation page.
export type AppView = AppPage | "welcome";

type AppHeaderProps = {
  currentPage?: AppPage;
  onNavigate: (page: AppPage) => void;
};

const pages: { id: AppPage; label: string }[] = [
  { id: "home", label: "Home" },
  { id: "settings", label: "Settings" },
];

export default function AppHeader({ currentPage, onNavigate }: AppHeaderProps) {
  return (
    <header className="app-header">
      <p className="app-name">Outflow</p>
      <nav aria-label="Application navigation" className="app-navigation">
        {pages.map((page) => (
          <button
            aria-current={currentPage === page.id ? "page" : undefined}
            className="app-navigation-button"
            key={page.id}
            onClick={() => onNavigate(page.id)}
            type="button"
          >
            {page.label}
          </button>
        ))}
      </nav>
    </header>
  );
}
