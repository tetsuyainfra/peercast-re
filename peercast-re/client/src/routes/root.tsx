import MainLayout from "@/layouts/mainLayout"
import { Outlet } from "react-router-dom"

export default function Root() {
  return (
    <MainLayout>
      <Outlet />
    </MainLayout>
  )
}
