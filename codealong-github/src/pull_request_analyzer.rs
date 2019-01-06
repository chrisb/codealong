use git2::{Oid, Repository};

use codealong::{with_authentication, Config, DiffAnalyzer};

use crate::analyzed_pull_request::AnalyzedPullRequest;
use crate::error::{Error, Result};
use crate::pull_request::{PullRequest, Ref};

pub struct PullRequestAnalyzer<'a> {
    repo: &'a Repository,
    config: &'a Config,
    pr: PullRequest,
}

impl<'a> PullRequestAnalyzer<'a> {
    pub fn new(
        repo: &'a Repository,
        pr: PullRequest,
        config: &'a Config,
    ) -> PullRequestAnalyzer<'a> {
        PullRequestAnalyzer { repo, pr, config }
    }

    pub fn analyze(self) -> Result<AnalyzedPullRequest> {
        self.fetch_remote(&self.pr.base)?;
        self.fetch_remote(&self.pr.head)?;

        let diff = self
            .repo
            .find_commit(Oid::from_str(&self.pr.base.sha)?)
            .map_err::<Error, _>(|e| e.into())
            .and_then(|parent| {
                self.repo
                    .find_commit(Oid::from_str(&self.pr.head.sha)?)
                    .map_err::<Error, _>(|e| e.into())
                    .and_then(|commit| {
                        Ok(
                            DiffAnalyzer::new(&self.repo, &commit, Some(&parent), &self.config)
                                .analyze()?,
                        )
                    })
            })
            .ok();

        Ok(AnalyzedPullRequest::new(self.pr, diff))
    }

    fn fetch_remote(&self, reference: &Ref) -> Result<()> {
        if let Some(ref repo) = reference.repo {
            let git_config = git2::Config::open_default()?;
            let url = &repo.ssh_url;
            with_authentication(url, &git_config, |f| {
                let mut rcb = git2::RemoteCallbacks::new();
                rcb.credentials(f);
                let mut fo = git2::FetchOptions::new();
                fo.remote_callbacks(rcb);

                Ok(self.repo.remote_anonymous(url).and_then(|mut base| {
                    base.fetch(&[&reference.reference], Some(&mut fo), None)
                })?)
            })?;
        }
        Ok(())
    }
}
