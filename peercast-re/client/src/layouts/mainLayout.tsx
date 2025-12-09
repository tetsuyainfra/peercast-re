interface MainLayoutProps extends React.HTMLAttributes<HTMLDivElement> {}

export default function MainLayout({ children }: MainLayoutProps) {
  return (
    <div className="hidden md:block">
      <div className="border-t">
        <div className="bg-background">
          <div className="grid lg:grid-cols-5">
            {/* <Sidebar playlists={playlists} className="hidden lg:block" /> */}
            <div className="col-span-3 lg:col-span-4 lg:border-l">
              <div className="h-full px-4 py-6 lg:px-8">{children}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
