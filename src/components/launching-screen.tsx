"use client";

import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import { Logo } from "@/components/icons/logo";

export function LaunchingScreen() {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-background select-none">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
        className="flex flex-col items-center"
      >
        <div className="relative mb-8">
          <Logo className="w-24 h-24 text-primary" />
          <motion.div
            className="absolute -inset-4 border-2 border-primary/20 rounded-full"
            animate={{
              scale: [1, 1.1, 1],
              opacity: [0.2, 0.5, 0.2],
            }}
            transition={{
              duration: 3,
              repeat: Infinity,
              ease: "easeInOut",
            }}
          />
        </div>

        <h1 className="text-2xl font-bold tracking-tight mb-2">Foxia-MKT</h1>

        <div className="flex flex-col items-center gap-3">
          <div className="flex gap-1">
            <motion.span
              animate={{ opacity: [0, 1, 0] }}
              transition={{ duration: 1.5, repeat: Infinity, delay: 0 }}
              className="w-1.5 h-1.5 rounded-full bg-primary"
            />
            <motion.span
              animate={{ opacity: [0, 1, 0] }}
              transition={{ duration: 1.5, repeat: Infinity, delay: 0.2 }}
              className="w-1.5 h-1.5 rounded-full bg-primary"
            />
            <motion.span
              animate={{ opacity: [0, 1, 0] }}
              transition={{ duration: 1.5, repeat: Infinity, delay: 0.4 }}
              className="w-1.5 h-1.5 rounded-full bg-primary"
            />
          </div>
          <p className="text-sm text-muted-foreground animate-pulse">
            {t("auth.checking_status", "Đang khởi động...")}
          </p>
        </div>
      </motion.div>
    </div>
  );
}
