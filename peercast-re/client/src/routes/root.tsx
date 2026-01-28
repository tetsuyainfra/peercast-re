import MainLayout from "@/layouts/mainLayout"
import { Outlet } from "react-router"

export default function Root() {
  return (
    <MainLayout>
      <Outlet />
    </MainLayout>
  )
}
