; ModuleID = 'runtime.c'
source_filename = "runtime.c"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-windows-msvc19.50.35725"

; Function Attrs: cold minsize noreturn nounwind uwtable
define dso_local void @_panic(ptr noundef readonly captures(none) %msg) local_unnamed_addr #0 {
entry:
  %call = tail call ptr @__acrt_iob_func(i32 noundef 2) #5
  %fputs = tail call i32 @fputs(ptr %msg, ptr %call)
  tail call void @exit(i32 noundef 1) #6
  unreachable
}

declare dso_local ptr @__acrt_iob_func(i32 noundef) local_unnamed_addr #1

; Function Attrs: nofree noreturn
declare dso_local void @exit(i32 noundef) local_unnamed_addr #2

; Function Attrs: nounwind uwtable
define dso_local noundef zeroext i1 @_array_equals(ptr noundef readonly captures(none) %lhs, ptr noundef readonly captures(none) %rhs, ptr noundef readonly captures(none) %elem_equals, i64 noundef %elem_size) local_unnamed_addr #3 {
entry:
  %0 = load ptr, ptr %lhs, align 8
  %1 = load ptr, ptr %rhs, align 8
  %cmp = icmp eq ptr %0, %1
  br i1 %cmp, label %cleanup19, label %if.end

if.end:                                           ; preds = %entry
  %cmp.i = icmp eq ptr %0, null
  %add.ptr.i = getelementptr inbounds i8, ptr %0, i64 -24
  %retval.0.i = select i1 %cmp.i, ptr null, ptr %add.ptr.i
  %count = getelementptr inbounds nuw i8, ptr %retval.0.i, i64 8
  %2 = load i64, ptr %count, align 8
  %count3 = getelementptr inbounds i8, ptr %1, i64 -16
  %3 = load i64, ptr %count3, align 8
  %cmp4.not = icmp eq i64 %2, %3
  br i1 %cmp4.not, label %for.cond.preheader, label %cleanup19

for.cond.preheader:                               ; preds = %if.end
  %cmp8.not36 = icmp eq i64 %2, 0
  br i1 %cmp8.not36, label %cleanup19, label %for.body

for.cond:                                         ; preds = %for.body
  %inc = add nuw i64 %i.037, 1
  %4 = load i64, ptr %count, align 8
  %cmp8.not.not = icmp ult i64 %inc, %4
  br i1 %cmp8.not.not, label %for.body, label %cleanup19, !llvm.loop !8

for.body:                                         ; preds = %for.cond.preheader, %for.cond
  %i.037 = phi i64 [ %inc, %for.cond ], [ 0, %for.cond.preheader ]
  %mul = mul i64 %i.037, %elem_size
  %arrayidx = getelementptr inbounds nuw i8, ptr %0, i64 %mul
  %arrayidx10 = getelementptr inbounds nuw i8, ptr %1, i64 %mul
  %call11 = tail call zeroext i1 %elem_equals(ptr noundef %arrayidx, ptr noundef %arrayidx10) #5
  br i1 %call11, label %for.cond, label %cleanup19

cleanup19:                                        ; preds = %for.cond, %for.body, %for.cond.preheader, %if.end, %entry
  %retval.4 = phi i1 [ true, %entry ], [ false, %if.end ], [ true, %for.cond.preheader ], [ %call11, %for.body ], [ %call11, %for.cond ]
  ret i1 %retval.4
}

; Function Attrs: nofree nounwind
declare noundef i32 @fputs(ptr noundef readonly captures(none), ptr noundef captures(none)) local_unnamed_addr #4

attributes #0 = { cold minsize noreturn nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nofree noreturn "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #4 = { nofree nounwind }
attributes #5 = { nounwind }
attributes #6 = { cold noreturn nounwind }

!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!2, !3, !4, !5, !6}
!llvm.ident = !{!7}

!0 = distinct !DICompileUnit(language: DW_LANG_C11, file: !1, producer: "clang version 21.1.6", isOptimized: true, runtimeVersion: 0, emissionKind: NoDebug, splitDebugInlining: false, nameTableKind: None)
!1 = !DIFile(filename: "runtime.c", directory: "C:\\Users\\acfro\\Documents\\Programming\\Languages\\patina")
!2 = !{i32 2, !"Debug Info Version", i32 3}
!3 = !{i32 1, !"wchar_size", i32 2}
!4 = !{i32 8, !"PIC Level", i32 2}
!5 = !{i32 7, !"uwtable", i32 2}
!6 = !{i32 1, !"MaxTLSAlign", i32 65536}
!7 = !{!"clang version 21.1.6"}
!8 = distinct !{!8, !9}
!9 = !{!"llvm.loop.mustprogress"}
