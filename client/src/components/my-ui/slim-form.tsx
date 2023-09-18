import { cn } from "@/lib/utils"
import React from "react"

type SlimFormItemContextValue = {
  id: string
}

const SlimFormItemContext = React.createContext<SlimFormItemContextValue>(
  {} as SlimFormItemContextValue,
)

const SlimFormItem = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => {
    const id = React.useId()

    return (
      <SlimFormItemContext.Provider value={{ id }}>
        <div ref={ref} className={cn(className)} {...props} />
      </SlimFormItemContext.Provider>
    )
  },
)
SlimFormItem.displayName = "SlimFormItem"

export { SlimFormItem }
